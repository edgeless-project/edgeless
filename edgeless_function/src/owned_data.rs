// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub struct OwnedByteBuff {
    pub(crate) data: *mut u8,
    pub(crate) size: usize,
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
            crate::memory::edgeless_mem_free(self.data, self.size);
        }
    }
}

impl OwnedByteBuff {
    pub(crate) unsafe fn new(ptr: *mut u8, len: usize) -> Self {
        Self { data: ptr, size: len }
    }

    pub fn new_from_slice(data: &[u8]) -> Self {
        unsafe {
            let ptr = crate::memory::edgeless_mem_alloc(data.len());
            core::slice::from_raw_parts_mut(ptr, data.len()).copy_from_slice(data);
            Self { data: ptr, size: data.len() }
        }
    }

    /// # Safety
    ///
    /// We are nice people. Nice people do not add bugs to unsafe Rust code.
    pub unsafe fn consume(self) -> (*mut u8, usize) {
        let res = (self.data, self.size);
        core::mem::forget(self);
        res
    }
}
