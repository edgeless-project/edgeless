// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub trait ContainerRuntimeAPI: Sync {
    fn guest_api_host(&mut self) -> Box<dyn crate::guest_api_host::GuestAPIHost>;
}
