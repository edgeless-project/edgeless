// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use std::time::UNIX_EPOCH;

use wasmtime::AsContextMut;

/// Binds the WASM component's imports to the function's GuestAPIHost. Functions
/// in this module are called directly by the Rust code for edgeless functions.
pub struct GuestAPI {
    // refers to the base runtime of an edgeless_node, one that is shared
    // between all runners and contains shared functionality, such as e.g.
    // dataplane calling
    pub host: crate::base_runtime::guest_api::GuestAPIHost,
}

pub async fn telemetry_log(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    level: i32,
    target_ptr: i32,
    target_len: i32,
    msg_ptr: i32,
    msg_len: i32,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;
    let target = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let msg = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, msg_ptr, msg_len)?;

    caller
        .data_mut()
        .host
        .telemetry_log(super::helpers::level_from_i32(level), &target, &msg)
        .await;
    Ok(())
}

pub async fn cast_raw(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    instance_node_id_ptr: i32,
    instance_component_id_ptr: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;
    let node_id = mem.data_mut(&mut caller)[instance_node_id_ptr as usize..(instance_node_id_ptr as usize) + 16_usize].to_vec();
    let component_id = mem.data_mut(&mut caller)[instance_component_id_ptr as usize..(instance_component_id_ptr as usize) + 16_usize].to_vec();
    let instance_id = edgeless_api::function_instance::InstanceId {
        node_id: uuid::Uuid::from_bytes(node_id.try_into().map_err(|_| wasmtime::Error::msg("uuid error"))?),
        function_id: uuid::Uuid::from_bytes(component_id.try_into().map_err(|_| wasmtime::Error::msg("uuid error"))?),
    };
    let payload = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    caller
        .data_mut()
        .host
        .cast_raw(instance_id, &payload)
        .await
        .map_err(|_| wasmtime::Error::msg("string error"))?;
    Ok(())
}

pub async fn call_raw(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    instance_node_id_ptr: i32,
    instance_component_id_ptr: i32,
    payload_ptr: i32,
    payload_len: i32,
    out_ptr_ptr: i32,
    out_len_ptr: i32,
) -> wasmtime::Result<i32> {
    let mem = get_memory(&mut caller)?;
    let alloc = get_alloc(&mut caller)?;
    let node_id = mem.data_mut(&mut caller)[instance_node_id_ptr as usize..(instance_node_id_ptr as usize) + 16_usize].to_vec();
    let component_id = mem.data_mut(&mut caller)[instance_component_id_ptr as usize..(instance_component_id_ptr as usize) + 16_usize].to_vec();
    let instance_id = edgeless_api::function_instance::InstanceId {
        node_id: uuid::Uuid::from_bytes(node_id.try_into().map_err(|_| wasmtime::Error::msg("uuid error"))?),
        function_id: uuid::Uuid::from_bytes(component_id.try_into().map_err(|_| wasmtime::Error::msg("uuid error"))?),
    };
    let payload = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    let call_ret = caller
        .data_mut()
        .host
        .call_raw(instance_id, &payload)
        .await
        .map_err(|_| wasmtime::Error::msg("call error"))?;
    match call_ret {
        edgeless_dataplane::core::CallRet::NoReply => Ok(0),
        edgeless_dataplane::core::CallRet::Reply(data) => {
            let len = data.len();

            let data_ptr = super::helpers::copy_to_vm(&mut caller.as_context_mut(), &mem, &alloc, data.as_bytes()).await?;
            super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_ptr_ptr, &data_ptr.to_le_bytes())?;
            super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_len_ptr, &len.to_le_bytes())?;

            Ok(1)
        }
        // TODO: copy the error string into the function
        edgeless_dataplane::core::CallRet::Err(_) => Ok(2),
    }
}

pub async fn cast(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;

    let target = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let payload = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    return match caller.data_mut().host.cast_alias(&target, &payload).await {
        Ok(_) => Ok(()),
        Err(failure) => match failure {
            crate::base_runtime::guest_api::GuestAPIError::UnknownAlias => {
                log::warn!("GuestAPI: Cast to unknown target: {}", target);
                Err(anyhow::Error::msg("Unknown alias").into())
            }
            crate::base_runtime::guest_api::GuestAPIError::Timeout => {
                log::warn!("GuestAPI: timeout: {}", target);
                Err(anyhow::Error::msg(format!("GuestAPI: timeout: {}", target)).into())
            }
            crate::base_runtime::guest_api::GuestAPIError::PoisonPill => {
                log::warn!("GuestAPI: poison pill: {}", target);
                Err(anyhow::Error::msg(format!("GuestAPI: poison pill: {}", target)).into())
            }
        },
    };
}

pub async fn call(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
    out_ptr_ptr: i32,
    out_len_ptr: i32,
) -> wasmtime::Result<i32> {
    let mem = get_memory(&mut caller)?;
    let alloc = get_alloc(&mut caller)?;

    // ? should only be used for functions that have nothing to do with the
    // dataplane (e.g. copy to vm etc.) - dataplane errors should be handled explicitly!

    let target = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let payload = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;
    let call_ret = caller.data_mut().host.call_alias(&target, &payload).await;

    // the question mark here made the call just propagate the error back to the
    // caller, instead of ever returning from the call
    // NOTE: if Ok is returned, this indicates a recoverable error, Err makes
    // the function stop
    match call_ret {
        Ok(success) => match success {
            // NOTE: I guess the integers 0-2 are a convention for how the
            // results are returned to the wasm module
            edgeless_dataplane::core::CallRet::NoReply => Ok(0),
            edgeless_dataplane::core::CallRet::Reply(data) => {
                let len = data.len();

                let data_ptr = super::helpers::copy_to_vm(&mut caller.as_context_mut(), &mem, &alloc, data.as_bytes()).await?;
                super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_ptr_ptr, &data_ptr.to_le_bytes())?;
                super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_len_ptr, &len.to_le_bytes())?;

                Ok(1)
            }
            // TODO: copy the error string to the function like in CallRet::Reply
            edgeless_dataplane::core::CallRet::Err(_) => Ok(2), // Err(anyhow::Error::msg("Dataplane CallRet:Err").into()),
        },
        // NOTE: very verbose and explicit error matching
        // NOTE: anyhow errors will result in InternalError -> BadCode, which
        // will stop the function; timeout will not stop the function
        Err(failure) => match failure {
            crate::base_runtime::guest_api::GuestAPIError::UnknownAlias => Err(anyhow::Error::msg("Unknown alias").into()),
            crate::base_runtime::guest_api::GuestAPIError::Timeout => {
                // as in above todo
                Ok(2)
            }
            crate::base_runtime::guest_api::GuestAPIError::PoisonPill => Err(anyhow::Error::msg("Poison pill").into()),
        },
    }
}

pub async fn delayed_cast(
    mut caller: wasmtime::Caller<'_, GuestAPI>,
    delay_ms: i64,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;
    let target = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let payload = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    caller
        .data_mut()
        .host
        .delayed_cast(delay_ms as u64, &target, &payload)
        .await
        .map_err(|_| wasmtime::Error::msg("call error"))?;
    Ok(())
}

pub async fn sync(mut caller: wasmtime::Caller<'_, GuestAPI>, state_ptr: i32, state_len: i32) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;
    let state = super::helpers::load_string_from_vm(&mut caller.as_context_mut(), &mem, state_ptr, state_len)?;

    caller
        .data_mut()
        .host
        .sync(&state)
        .await
        .map_err(|_| wasmtime::Error::msg("sync error"))?;
    Ok(())
}

pub async fn get_current_time(mut _caller: wasmtime::Caller<'_, GuestAPI>) -> wasmtime::Result<(u64)> {
    let now = std::time::SystemTime::now();
    let timestamp = now.duration_since(std::time::UNIX_EPOCH).expect("time went backwards").as_nanos() as u64;
    Ok(timestamp)
}

pub async fn slf(mut caller: wasmtime::Caller<'_, GuestAPI>, out_node_id_ptr: i32, out_component_id_ptr: i32) -> wasmtime::Result<()> {
    let mem = get_memory(&mut caller)?;

    let id = caller.data_mut().host.slf().await;

    super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_node_id_ptr, id.node_id.as_bytes())?;
    super::helpers::copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_component_id_ptr, id.function_id.as_bytes())?;

    Ok(())
}

pub(crate) fn get_memory(caller: &mut wasmtime::Caller<'_, super::guest_api_binding::GuestAPI>) -> wasmtime::Result<wasmtime::Memory> {
    caller
        .get_export("memory")
        .ok_or(wasmtime::Error::msg("memory error"))?
        .into_memory()
        .ok_or(wasmtime::Error::msg("memory error"))
}

pub(crate) fn get_alloc(caller: &mut wasmtime::Caller<'_, super::guest_api_binding::GuestAPI>) -> wasmtime::Result<wasmtime::TypedFunc<i32, i32>> {
    caller
        .get_export("edgeless_mem_alloc")
        .ok_or(wasmtime::Error::msg("alloc error"))?
        .into_func()
        .ok_or(wasmtime::Error::msg("alloc error"))?
        .typed::<i32, i32>(&caller)
        .map_err(|_| wasmtime::Error::msg("alloc error"))
}
