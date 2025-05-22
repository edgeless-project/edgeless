// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub trait OrchestratorAPI: Send {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceAPI<crate::function_instance::DomainManagedInstanceId>>;
    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn crate::resource_configuration::ResourceConfigurationAPI<crate::function_instance::DomainManagedInstanceId>>;
}
