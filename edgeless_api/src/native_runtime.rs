// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

pub trait NativeRuntimeAPI: Sync {
    fn guest_api_host(&mut self) -> Box<dyn crate::guest_api_host::GuestAPIHost>;
    
    /*#[no_mangle]
    unsafe extern "C" fn telemetry_log_asm (
        &mut self,
        level: usize, 
        target_ptr: *const u8, 
        target_len: usize, 
        msg_ptr: *const u8, 
        msg_len: usize,
    );*/
}
