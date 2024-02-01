// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use std::borrow::BorrowMut;

pub(crate) fn copy_to_vm(
    ctx: &mut wasmi::StoreContextMut<'_, super::guest_api_binding::GuestAPI>,
    memory: &wasmi::Memory,
    alloc: &wasmi::TypedFunc<i32, i32>,
    data: &[u8],
) -> Result<i32, wasmi::core::Trap> {
    let data_ptr = alloc
        .call(ctx.borrow_mut(), data.len() as i32)
        .map_err(|_| wasmi::core::Trap::new("alloc error"))?;
    memory.data_mut(ctx.borrow_mut())[data_ptr as usize..(data_ptr as usize) + data.len()].copy_from_slice(data);
    Ok(data_ptr)
}

// This does not check the target length
pub(crate) fn copy_to_vm_ptr(
    ctx: &mut wasmi::StoreContextMut<'_, super::guest_api_binding::GuestAPI>,
    memory: &wasmi::Memory,
    target_ptr: i32,
    data: &[u8],
) -> Result<(), wasmi::core::Trap> {
    memory.data_mut(ctx.borrow_mut())[target_ptr as usize..(target_ptr as usize) + data.len()].copy_from_slice(data);
    Ok(())
}

pub(crate) fn load_string_from_vm(
    ctx: &mut wasmi::StoreContextMut<'_, super::guest_api_binding::GuestAPI>,
    memory: &wasmi::Memory,
    data_ptr: i32,
    data_len: i32,
) -> Result<String, wasmi::core::Trap> {
    String::from_utf8(memory.data_mut(ctx)[data_ptr as usize..(data_ptr as usize) + data_len as usize].to_vec())
        .map_err(|_| wasmi::core::Trap::new("string error"))
}

pub(crate) fn level_from_i32(lvl: i32) -> edgeless_telemetry::telemetry_events::TelemetryLogLevel {
    match lvl {
        1 => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Error,
        2 => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Warn,
        3 => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info,
        4 => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Debug,
        5 => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Trace,
        _ => {
            log::warn!("Function used unknown Log Level");
            edgeless_telemetry::telemetry_events::TelemetryLogLevel::Error
        }
    }
}
