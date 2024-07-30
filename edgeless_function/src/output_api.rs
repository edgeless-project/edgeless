// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub fn cast_raw(target: crate::InstanceId, port: &str, msg: &[u8]) {
    unsafe {
        crate::imports::cast_raw_asm(
            target.node_id.as_ptr(),
            target.component_id.as_ptr(),
            port.as_bytes().as_ptr(),
            port.as_bytes().len(),
            msg.as_ptr(),
            msg.len(),
        );
    }
}

pub fn cast(name: &str, msg: &[u8]) {
    unsafe {
        crate::imports::cast_asm(name.as_bytes().as_ptr(), name.as_bytes().len(), msg.as_ptr(), msg.len());
    }
}

pub fn delayed_cast(delay_ms: u64, name: &str, msg: &[u8]) {
    unsafe {
        crate::imports::delayed_cast_asm(delay_ms, name.as_bytes().as_ptr(), name.as_bytes().len(), msg.as_ptr(), msg.len());
    }
}

pub fn call_raw(target: crate::InstanceId, port: &str, msg: &[u8]) -> crate::CallRet {
    unsafe {
        let mut out_ptr_ptr: *mut u8 = core::ptr::null_mut();
        let mut out_len_ptr: usize = 0;

        let call_ret_type = crate::imports::call_raw_asm(
            target.node_id.as_ptr(),
            target.component_id.as_ptr(),
            port.as_bytes().as_ptr(),
            port.as_bytes().len(),
            msg.as_ptr(),
            msg.len(),
            &mut out_ptr_ptr as *mut *mut u8,
            &mut out_len_ptr as *mut usize,
        );

        match call_ret_type {
            0 => crate::CallRet::NoReply,
            1 => crate::CallRet::Reply(crate::owned_data::OwnedByteBuff::new(out_ptr_ptr, out_len_ptr)),
            _ => crate::CallRet::Err,
        }
    }
}

pub fn call(name: &str, msg: &[u8]) -> crate::CallRet {
    unsafe {
        let mut out_ptr_ptr: *mut u8 = core::ptr::null_mut();
        let mut out_len_ptr: usize = 0;
        let call_ret_type = crate::imports::call_asm(
            name.as_bytes().as_ptr(),
            name.as_bytes().len(),
            msg.as_ptr(),
            msg.len(),
            &mut out_ptr_ptr as *mut *mut u8,
            &mut out_len_ptr as *mut usize,
        );

        match call_ret_type {
            0 => crate::CallRet::NoReply,
            1 => crate::CallRet::Reply(crate::owned_data::OwnedByteBuff::new(out_ptr_ptr, out_len_ptr)),
            _ => crate::CallRet::Err,
        }
    }
}

pub fn telemetry_log(level: usize, target: &str, msg: &str) {
    unsafe {
        crate::imports::telemetry_log_asm(
            level,
            target.as_bytes().as_ptr(),
            target.as_bytes().len(),
            msg.as_bytes().as_ptr(),
            msg.len(),
        );
    }
}

pub fn slf() -> crate::InstanceId {
    unsafe {
        let mut id = crate::InstanceId {
            node_id: [0; 16],
            component_id: [0; 16],
        };
        crate::imports::slf_asm(id.node_id.as_mut_ptr(), id.component_id.as_mut_ptr());
        id
    }
}

pub fn sync(state: &[u8]) {
    unsafe {
        crate::imports::sync_asm(state.as_ptr(), state.len() as u32);
    }
}
