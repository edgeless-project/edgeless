// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub mod lcg;

pub mod export;

pub enum CallRet {
    NoReply,
    Reply(OwnedByteBuff),
    Err,
}

pub struct OwnedByteBuff {
    data: *mut u8,
    size: usize,
}

impl core::ops::Deref for OwnedByteBuff {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.data, self.size) }
    }
}

impl core::ops::DerefMut for OwnedByteBuff {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.data, self.size) }
    }
}

impl Drop for OwnedByteBuff {
    fn drop(&mut self) {
        unsafe {
            edgeless_mem_free(self.data, self.size);
        }
    }
}

impl OwnedByteBuff {
    unsafe fn new(ptr: *mut u8, len: usize) -> Self {
        Self { data: ptr, size: len }
    }

    pub fn new_from_slice(data: &[u8]) -> Self {
        unsafe {
            let ptr = edgeless_mem_alloc(data.len());
            core::slice::from_raw_parts_mut(ptr, data.len()).copy_from_slice(data);
            Self { data: ptr, size: data.len() }
        }
    }

    pub unsafe fn consume(self) -> (*mut u8, usize) {
        let res = (self.data, self.size);
        core::mem::drop(self);
        res
    }
}

pub struct InstanceId {
    pub node_id: [u8; 16],
    pub component_id: [u8; 16],
}

pub trait EdgeFunction {
    fn handle_cast(src: InstanceId, encoded_message: &[u8]);
    fn handle_call(src: InstanceId, encoded_message: &[u8]) -> CallRet;
    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>);
    fn handle_stop();
}

pub fn cast_raw(target: InstanceId, msg: &[u8]) {
    unsafe {
        cast_raw_asm(
            target.node_id.as_ptr(),
            target.component_id.as_ptr(),
            msg.as_ptr(),
            msg.len(),
        );
    }
}

pub fn cast(name: &str, msg: &[u8]) {
    unsafe {
        cast_asm(
            name.as_bytes().as_ptr(),
            name.as_bytes().len(),
            msg.as_ptr(),
            msg.len(),
        );
    }
}

pub fn delayed_cast(delay_ms: u64, name: &str, msg: &[u8]) {
    unsafe {
        delayed_cast_asm(
            delay_ms,
            name.as_bytes().as_ptr(),
            name.as_bytes().len(),
            msg.as_ptr(),
            msg.len(),
        );
    }
}

pub fn call_raw(target: InstanceId, msg: &[u8]) -> CallRet {
    unsafe {
        let mut out_ptr_ptr: *mut u8 = core::ptr::null_mut();
        let mut out_len_ptr: usize = 0;

        let call_ret_type = call_raw_asm(
            target.node_id.as_ptr(),
            target.component_id.as_ptr(),
            msg.as_ptr(),
            msg.len(),
            &mut out_ptr_ptr as *mut *mut u8,
            &mut out_len_ptr as *mut usize,
        );

        match call_ret_type {
            0 => CallRet::NoReply,
            1 => CallRet::Reply(OwnedByteBuff::new(out_ptr_ptr, out_len_ptr)),
            _ => CallRet::Err,
        }
    }
}

pub fn call(name: &str, msg: &[u8]) -> CallRet {
    unsafe {
        let mut out_ptr_ptr: *mut u8 = core::ptr::null_mut();
        let mut out_len_ptr: usize = 0;
        let call_ret_type = call_asm(
            name.as_bytes().as_ptr(),
            name.as_bytes().len(),
            msg.as_ptr(),
            msg.len(),
            &mut out_ptr_ptr as *mut *mut u8,
            &mut out_len_ptr as *mut usize,
        );

        match call_ret_type {
            0 => CallRet::NoReply,
            1 => CallRet::Reply(OwnedByteBuff::new(out_ptr_ptr, out_len_ptr)),
            _ => CallRet::Err,
        }
    }
}

pub fn telemetry_log(level: usize, target: &str, msg: &str) {
    unsafe {
        telemetry_log_asm(
            level,
            target.as_bytes().as_ptr(),
            target.as_bytes().len(),
            msg.as_bytes().as_ptr(),
            msg.len(),
        );
    }
}

pub fn slf() -> InstanceId {
    unsafe {
        let mut id = InstanceId {
            node_id: [0; 16],
            component_id: [0; 16],
        };
        slf_asm(id.node_id.as_mut_ptr(), id.component_id.as_mut_ptr());
        id
    }
}

pub fn sync(state: &[u8]) {
    unsafe {
        sync_asm(state.as_ptr(), state.len() as u32);
    }
}

extern "C" {
    fn cast_raw_asm(instance_node_id_ptr: *const u8, instance_component_id_ptr: *const u8, payload_ptr: *const u8, payload_len: usize);
    fn cast_asm(target_ptr: *const u8, target_len: usize, payload_ptr: *const u8, payload_len: usize);
    fn call_raw_asm(
        instance_node_id_ptr: *const u8,
        instance_component_id_ptr: *const u8,
        payload_ptr: *const u8,
        payload_len: usize,
        out_ptr_ptr: *mut *mut u8,
        out_len_ptr: *mut usize,
    ) -> i32;
    fn call_asm(
        target_ptr: *const u8,
        target_len: usize,
        payload_ptr: *const u8,
        payload_len: usize,
        out_ptr_ptr: *mut *mut u8,
        out_len_ptr: *mut usize,
    ) -> i32;
    fn telemetry_log_asm(level: usize, target_ptr: *const u8, target_len: usize, msg_ptr: *const u8, msg_len: usize);
    fn slf_asm(out_node_id_ptr: *mut u8, out_component_id_ptr: *mut u8);
    fn delayed_cast_asm(delay_ms: u64, target_ptr: *const u8, target_len: usize, payload_ptr: *const u8, payload_len: usize);
    fn sync_asm(data_ptr: *const u8, data_len: u32);
}

#[cfg(not(feature = "std"))]
static mut data_arena: [u8; 32 * 1024] = [0; 32 * 1024];
#[cfg(not(feature = "std"))]
static mut arena_start: usize = 0;

#[cfg(not(feature = "std"))]
#[no_mangle]
pub unsafe extern "C" fn edgeless_mem_alloc(payload_len: usize) -> *mut u8 {
    let stop = arena_start + payload_len;
    if stop > data_arena.len() {
        panic!("OOM");
    }
    let out = data_arena[arena_start..stop].as_mut_ptr();
    arena_start = stop;
    out
}

#[cfg(feature = "std")]
#[no_mangle]
// https://radu-matei.com/blog/practical-guide-to-wasm-memory/
pub unsafe extern "C" fn edgeless_mem_alloc(payload_len: usize) -> *mut u8 {
    let align = std::mem::align_of::<usize>();
    let layout = std::alloc::Layout::from_size_align_unchecked(payload_len, align);
    std::alloc::alloc(layout)
}

#[cfg(not(feature = "std"))]
#[no_mangle]
pub unsafe extern "C" fn edgeless_mem_clear() {
    arena_start = 0;
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn edgeless_mem_clear() {
    // We always free and clear, so this does not leak memory.
}

#[cfg(not(feature = "std"))]
#[no_mangle]
pub unsafe extern "C" fn edgeless_mem_free(ptr: *mut u8, size: usize) {
    // We always free and clear, so this does not leak memory.
}

#[cfg(feature = "std")]
#[no_mangle]
pub unsafe extern "C" fn edgeless_mem_free(ptr: *mut u8, size: usize) {
    let align = std::mem::align_of::<usize>();
    let layout = std::alloc::Layout::from_size_align_unchecked(size, align);
    std::alloc::dealloc(ptr, layout);
}

pub fn rust_to_api(lvl: log::Level) -> u32 {
    match lvl {
        log::Level::Error => 1,
        log::Level::Warn => 2,
        log::Level::Info => 3,
        log::Level::Debug => 4,
        log::Level::Trace => 5,
    }
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    #[cfg(not(feature = "std"))]
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            match record.args().as_str() {
                Some(data) => telemetry_log(rust_to_api(record.level()) as usize, record.target(), data),
                _ => {
                    telemetry_log(rust_to_api(record.level()) as usize, record.target(), "Unsupported Message Arguments");
                }
            }
        }
    }
    #[cfg(feature = "std")]
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            telemetry_log(rust_to_api(record.level()) as usize, record.target(), &record.args().to_string());
        }
    }

    fn flush(&self) {}
}

static LOGGER: Logger = Logger;

pub fn init_logger() {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Debug)).unwrap();
}

#[cfg(feature = "std")]
pub fn parse_init_payload(payload: &str) -> std::collections::HashMap<&str, &str> {
    let tokens = payload.split(',');
    let mut arguments = std::collections::HashMap::new();
    for token in tokens {
        let mut inner_tokens = token.split('=');
        if let Some(key) = inner_tokens.next() {
            if let Some(value) = inner_tokens.next() {
                arguments.insert(key, value);
            } else {
                log::error!("invalid initialization token: {}", token);
            }
        } else {
            log::error!("invalid initialization token: {}", token);
        }
    }
    arguments
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(feature = "std")]
    #[test]
    fn test_parse_init_payload() {
        assert_eq!(
            std::collections::HashMap::from([("a", "b"), ("c", "d"), ("my_key", "my_value")]),
            parse_init_payload("a=b,c=d,my_key=my_value")
        );

        assert_eq!(
            std::collections::HashMap::from([("a", ""), ("", "d"), ("my_key", "my_value")]),
            parse_init_payload("a=,=d,my_key=my_value")
        );

        assert_eq!(
            std::collections::HashMap::from([("my_key", "my_value")]),
            parse_init_payload("a,d,my_key=my_value")
        );

        assert!(parse_init_payload(",,,a,s,s,,42,").is_empty());
    }
}
