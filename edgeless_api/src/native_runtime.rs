// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

pub trait NativeRuntimeAPI: Sync {
    fn guest_api_host(&mut self) -> Box<dyn crate::guest_api_host::GuestAPIHost>;
}
