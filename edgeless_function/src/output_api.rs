// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[cfg(target_arch = "x86_64")]
use {
    once_cell::sync::OnceCell,
    core::sync::atomic::{AtomicPtr, Ordering},
};

#[cfg(target_arch = "x86_64")]
static GUEST_API_HOST: OnceCell<AtomicPtr<usize>> = OnceCell::new();

#[cfg(target_arch = "x86_64")]
#[no_mangle]
pub extern "C" fn set_guest_api_host_pointer(ptr: *const usize) {
    println!("Setting the GUEST_API_HOST pointer to: {:p}", ptr);
    
    let atomic_ptr = AtomicPtr::new(ptr as *mut usize);
    GUEST_API_HOST.set(atomic_ptr).unwrap();
}

#[cfg(target_arch = "x86_64")]
fn get_guest_api_host_pointer() -> *const usize {
    let ptr = GUEST_API_HOST.get().expect("Guest API Host not set up");
    ptr.load(Ordering::SeqCst)
}

#[cfg(target_arch = "wasm32")]
pub fn cast_raw(target: crate::InstanceId, msg: &[u8]) {
    unsafe {
        crate::imports::cast_raw_asm(target.node_id.as_ptr(), target.component_id.as_ptr(), msg.as_ptr(), msg.len());
    }
}
#[cfg(target_arch = "x86_64")]
pub fn cast_raw(target: crate::InstanceId, msg: &[u8]) {
    unsafe {
        let ptr = get_guest_api_host_pointer();
        crate::imports::cast_raw_asm(ptr, target.node_id.as_ptr(), target.component_id.as_ptr(), msg.as_ptr(), msg.len());
    }
}


#[cfg(target_arch = "wasm32")]
pub fn cast(name: &str, msg: &[u8]) {
    unsafe {
        crate::imports::cast_asm(name.as_bytes().as_ptr(), name.as_bytes().len(), msg.as_ptr(), msg.len());
    }
}

#[cfg(target_arch = "x86_64")]
pub fn cast(name: &str, msg: &[u8]) {
    unsafe {
        let ptr = get_guest_api_host_pointer();
        crate::imports::cast_asm(ptr, name.as_bytes().as_ptr(), name.as_bytes().len(), msg.as_ptr(), msg.len());
    }
}


#[cfg(target_arch = "wasm32")]
pub fn delayed_cast(delay_ms: u64, name: &str, msg: &[u8]) {
    unsafe {
        crate::imports::delayed_cast_asm(delay_ms, name.as_bytes().as_ptr(), name.as_bytes().len(), msg.as_ptr(), msg.len());
    }
}

#[cfg(target_arch = "x86_64")]
pub fn delayed_cast(delay_ms: u64, name: &str, msg: &[u8]) {
    unsafe {
        let ptr = get_guest_api_host_pointer();
        crate::imports::delayed_cast_asm(ptr, delay_ms, name.as_bytes().as_ptr(), name.as_bytes().len(), msg.as_ptr(), msg.len());
    }
}

#[cfg(target_arch = "wasm32")]
pub fn call_raw(target: crate::InstanceId, msg: &[u8]) -> crate::CallRet {
    unsafe {
        let mut out_ptr_ptr: *mut u8 = core::ptr::null_mut();
        let mut out_len_ptr: usize = 0;

        let call_ret_type = crate::imports::call_raw_asm(
            target.node_id.as_ptr(),
            target.component_id.as_ptr(),
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

#[cfg(target_arch = "x86_64")]
pub fn call_raw(target: crate::InstanceId, msg: &[u8]) -> crate::CallRet {
    unsafe {
        let ptr = get_guest_api_host_pointer();
        let mut out_ptr_ptr: *mut u8 = core::ptr::null_mut();
        let mut out_len_ptr: usize = 0;

        let call_ret_type = crate::imports::call_raw_asm(
            ptr,
            target.node_id.as_ptr(),
            target.component_id.as_ptr(),
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


#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "x86_64")]
pub fn call(name: &str, msg: &[u8]) -> crate::CallRet {
    unsafe {
        let ptr = get_guest_api_host_pointer();
        let mut out_ptr_ptr: *mut u8 = core::ptr::null_mut();
        let mut out_len_ptr: usize = 0;
        let call_ret_type = crate::imports::call_asm(
            ptr,
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

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "x86_64")]
pub fn telemetry_log(level: usize, target: &str, msg: &str) {
    unsafe {
        let ptr = get_guest_api_host_pointer();

        crate::imports::telemetry_log_asm(
            ptr, 
            level,
            target.as_bytes().as_ptr(),
            target.as_bytes().len(),
            msg.as_bytes().as_ptr(),
            msg.len(),
        );
    }
}


#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "x86_64")]
pub fn slf() -> crate::InstanceId {
    unsafe {
        let ptr = get_guest_api_host_pointer();
        let mut id = crate::InstanceId {
            node_id: [0; 16],
            component_id: [0; 16],
        };
        crate::imports::slf_asm(ptr, id.node_id.as_mut_ptr(), id.component_id.as_mut_ptr());
        id
    }
}


#[cfg(target_arch = "wasm32")]
pub fn sync(state: &[u8]) {
    unsafe {
        crate::imports::sync_asm(state.as_ptr(), state.len() as u32);
    }
}

#[cfg(target_arch = "x86_64")]
pub fn sync(state: &[u8]) {
    unsafe {
        let ptr = get_guest_api_host_pointer();
        crate::imports::sync_asm(ptr, state.as_ptr(), state.len() as u32);
    }
}
