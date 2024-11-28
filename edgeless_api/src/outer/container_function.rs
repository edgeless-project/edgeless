// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub trait ContainerFunctionAPI: Sync {
    fn guest_api_function(&mut self) -> Box<dyn crate::guest_api_function::GuestAPIFunction>;
}
