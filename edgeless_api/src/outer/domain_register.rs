// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub trait DomainRegisterAPI: Sync {
    fn domain_registration_api(&mut self) -> Box<dyn crate::domain_registration::DomainRegistrationAPI>;
}
