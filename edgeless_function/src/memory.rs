// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

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
#[unsafe(no_mangle)]
/// # Safety
///
/// https://radu-matei.com/blog/practical-guide-to-wasm-memory/
pub unsafe extern "C" fn edgeless_mem_alloc(payload_len: usize) -> *mut u8 { unsafe {
    let align = std::mem::align_of::<usize>();
    let layout = std::alloc::Layout::from_size_align_unchecked(payload_len, align);
    std::alloc::alloc(layout)
}}

#[cfg(not(feature = "std"))]
#[no_mangle]
pub unsafe extern "C" fn edgeless_mem_clear() {
    arena_start = 0;
}

#[cfg(feature = "std")]
#[unsafe(no_mangle)]
/// # Safety
///
/// We always free and clear, so this does not leak memory.
pub unsafe extern "C" fn edgeless_mem_clear() {}

#[cfg(not(feature = "std"))]
#[no_mangle]
/// # Safety
///
/// We always free and clear, so this does not leak memory.
pub unsafe extern "C" fn edgeless_mem_free(ptr: *mut u8, size: usize) {}

/// # Safety
///
/// We always free and clear, so this does not leak memory.
#[cfg(feature = "std")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgeless_mem_free(ptr: *mut u8, size: usize) { unsafe {
    let align = std::mem::align_of::<usize>();
    let layout = std::alloc::Layout::from_size_align_unchecked(size, align);
    std::alloc::dealloc(ptr, layout);
}}
