// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub trait NodeRegisterAPI: Send {
    fn node_registration_api(&mut self) -> Box<dyn crate::node_registration::NodeRegistrationAPI>;
}
