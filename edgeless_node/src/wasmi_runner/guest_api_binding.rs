// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use wasmi::AsContextMut;

use super::helpers::*;
pub struct GuestAPI {
    pub host: crate::base_runtime::guest_api::GuestAPIHost,
}

pub fn telemetry_log(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    level: i32,
    target_ptr: i32,
    target_len: i32,
    msg_ptr: i32,
    msg_len: i32,
) -> Result<(), wasmi::core::Trap> {
    let mem = get_memory(&mut caller)?;
    let target = load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let msg = load_string_from_vm(&mut caller.as_context_mut(), &mem, msg_ptr, msg_len)?;

    tokio::runtime::Handle::current().block_on(caller.data_mut().host.telemetry_log(level_from_i32(level), &target, &msg));
    Ok(())
}

pub fn cast_raw(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    instance_node_id_ptr: i32,
    instance_component_id_ptr: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> Result<(), wasmi::core::Trap> {
    let mem = get_memory(&mut caller)?;
    let node_id = mem.data_mut(&mut caller)[instance_node_id_ptr as usize..(instance_node_id_ptr as usize) + 16 as usize].to_vec();
    let component_id = mem.data_mut(&mut caller)[instance_component_id_ptr as usize..(instance_component_id_ptr as usize) + 16 as usize].to_vec();
    let instance_id = edgeless_api::function_instance::InstanceId {
        node_id: uuid::Uuid::from_bytes(node_id.try_into().map_err(|_| wasmi::core::Trap::new("uuid error"))?),
        function_id: uuid::Uuid::from_bytes(component_id.try_into().map_err(|_| wasmi::core::Trap::new("uuid error"))?),
    };
    let payload = load_string_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    tokio::runtime::Handle::current()
        .block_on(caller.data_mut().host.cast_raw(instance_id, &payload))
        .map_err(|_| wasmi::core::Trap::new("string error"))?;
    Ok(())
}

pub fn call_raw(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    instance_node_id_ptr: i32,
    instance_component_id_ptr: i32,
    payload_ptr: i32,
    payload_len: i32,
    out_ptr_ptr: i32,
    out_len_ptr: i32,
) -> Result<i32, wasmi::core::Trap> {
    let mem = get_memory(&mut caller)?;
    let alloc = get_alloc(&mut caller)?;
    let node_id = mem.data_mut(&mut caller)[instance_node_id_ptr as usize..(instance_node_id_ptr as usize) + 16 as usize].to_vec();
    let component_id = mem.data_mut(&mut caller)[instance_component_id_ptr as usize..(instance_component_id_ptr as usize) + 16 as usize].to_vec();
    let instance_id = edgeless_api::function_instance::InstanceId {
        node_id: uuid::Uuid::from_bytes(node_id.try_into().map_err(|_| wasmi::core::Trap::new("uuid error"))?),
        function_id: uuid::Uuid::from_bytes(component_id.try_into().map_err(|_| wasmi::core::Trap::new("uuid error"))?),
    };
    let payload = load_string_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    let call_ret = tokio::runtime::Handle::current()
        .block_on(caller.data_mut().host.call_raw(instance_id, &payload))
        .map_err(|_| wasmi::core::Trap::new("call error"))?;
    match call_ret {
        edgeless_dataplane::core::CallRet::NoReply => Ok(0),
        edgeless_dataplane::core::CallRet::Reply(data) => {
            let len = data.as_bytes().len();

            let data_ptr = copy_to_vm(&mut caller.as_context_mut(), &mem, &alloc, data.as_bytes())?;
            copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_ptr_ptr, &data_ptr.to_le_bytes())?;
            copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_len_ptr, &len.to_le_bytes())?;

            Ok(1)
        }
        edgeless_dataplane::core::CallRet::Err => Ok(2),
    }
}

pub fn cast(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> Result<(), wasmi::core::Trap> {
    let mem = get_memory(&mut caller)?;

    let target = load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let payload = load_string_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    match tokio::runtime::Handle::current().block_on(caller.data_mut().host.cast_alias(&target, &payload)) {
        Ok(_) => {}
        Err(_) => {
            // We ignore casts to unknown targets.
            log::warn!("Cast to unknown target");
        }
    };

    Ok(())
}

pub fn call(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
    out_ptr_ptr: i32,
    out_len_ptr: i32,
) -> Result<i32, wasmi::core::Trap> {
    let mem = get_memory(&mut caller)?;
    let alloc = get_alloc(&mut caller)?;

    let target = load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let payload = load_string_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    let call_ret = tokio::runtime::Handle::current()
        .block_on(caller.data_mut().host.call_alias(&target, &payload))
        .map_err(|_| wasmi::core::Trap::new("call error"))?;
    match call_ret {
        edgeless_dataplane::core::CallRet::NoReply => Ok(0),
        edgeless_dataplane::core::CallRet::Reply(data) => {
            let len = data.as_bytes().len();

            let data_ptr = copy_to_vm(&mut caller.as_context_mut(), &mem, &alloc, data.as_bytes())?;
            copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_ptr_ptr, &data_ptr.to_le_bytes())?;
            copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_len_ptr, &len.to_le_bytes())?;

            Ok(1)
        }
        edgeless_dataplane::core::CallRet::Err => Ok(2),
    }
}

pub fn delayed_cast(
    mut caller: wasmi::Caller<'_, GuestAPI>,
    delay_ms: i64,
    target_ptr: i32,
    target_len: i32,
    payload_ptr: i32,
    payload_len: i32,
) -> Result<(), wasmi::core::Trap> {
    let mem = get_memory(&mut caller)?;
    let target = load_string_from_vm(&mut caller.as_context_mut(), &mem, target_ptr, target_len)?;
    let payload = load_string_from_vm(&mut caller.as_context_mut(), &mem, payload_ptr, payload_len)?;

    tokio::runtime::Handle::current()
        .block_on(caller.data_mut().host.delayed_cast(delay_ms as u64, &target, &payload))
        .map_err(|_| wasmi::core::Trap::new("call error"))?;
    Ok(())
}

pub fn sync(mut caller: wasmi::Caller<'_, GuestAPI>, state_ptr: i32, state_len: i32) -> Result<(), wasmi::core::Trap> {
    let mem = get_memory(&mut caller)?;
    let state = load_string_from_vm(&mut caller.as_context_mut(), &mem, state_ptr, state_len)?;

    tokio::runtime::Handle::current()
        .block_on(caller.data_mut().host.sync(&state))
        .map_err(|_| wasmi::core::Trap::new("sync error"))?;
    Ok(())
}

pub fn slf(mut caller: wasmi::Caller<'_, GuestAPI>, out_node_id_ptr: i32, out_component_id_ptr: i32) -> Result<(), wasmi::core::Trap> {
    let mem = get_memory(&mut caller)?;

    let id = tokio::runtime::Handle::current().block_on(caller.data_mut().host.slf());

    copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_node_id_ptr, id.node_id.as_bytes())?;
    copy_to_vm_ptr(&mut caller.as_context_mut(), &mem, out_component_id_ptr, id.function_id.as_bytes())?;

    Ok(())
}

pub(crate) fn get_memory(caller: &mut wasmi::Caller<'_, super::guest_api_binding::GuestAPI>) -> Result<wasmi::Memory, wasmi::core::Trap> {
    caller
        .get_export("memory")
        .ok_or(wasmi::core::Trap::new("memory error"))?
        .into_memory()
        .ok_or(wasmi::core::Trap::new("memory error"))
}

pub(crate) fn get_alloc(caller: &mut wasmi::Caller<'_, super::guest_api_binding::GuestAPI>) -> Result<wasmi::TypedFunc<i32, i32>, wasmi::core::Trap> {
    caller
        .get_export("edgeless_mem_alloc")
        .ok_or(wasmi::core::Trap::new("alloc error"))?
        .into_func()
        .ok_or(wasmi::core::Trap::new("alloc error"))?
        .typed::<i32, i32>(&caller)
        .map_err(|_| wasmi::core::Trap::new("alloc error"))
}
