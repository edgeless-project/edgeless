// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// Raw WASM-host-provided output interface
unsafe extern "C" {
    pub(crate) fn cast_raw_asm(
        instance_node_id_ptr: *const u8,
        instance_component_id_ptr: *const u8,
        payload_ptr: *const u8,
        payload_len: usize,
    );
    pub(crate) fn cast_asm(
        target_ptr: *const u8,
        target_len: usize,
        payload_ptr: *const u8,
        payload_len: usize,
    );
    pub(crate) fn call_raw_asm(
        instance_node_id_ptr: *const u8,
        instance_component_id_ptr: *const u8,
        payload_ptr: *const u8,
        payload_len: usize,
        out_ptr_ptr: *mut *mut u8,
        out_len_ptr: *mut usize,
    ) -> i32;
    pub(crate) fn call_asm(
        target_ptr: *const u8,
        target_len: usize,
        payload_ptr: *const u8,
        payload_len: usize,
        out_ptr_ptr: *mut *mut u8,
        out_len_ptr: *mut usize,
    ) -> i32;
    pub(crate) fn telemetry_log_asm(
        level: usize,
        target_ptr: *const u8,
        target_len: usize,
        msg_ptr: *const u8,
        msg_len: usize,
    );
    pub(crate) fn slf_asm(out_node_id_ptr: *mut u8, out_component_id_ptr: *mut u8);
    pub(crate) fn delayed_cast_asm(
        delay_ms: u64,
        target_ptr: *const u8,
        target_len: usize,
        payload_ptr: *const u8,
        payload_len: usize,
    );
    pub(crate) fn sync_asm(data_ptr: *const u8, data_len: u32);
}
