// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
pub type DomainManagedInstanceId = uuid::Uuid;

pub trait OrchestratorAPI: Send {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceAPI<DomainManagedInstanceId>>;
    fn resource_configuration_api(&mut self) -> Box<dyn crate::resource_configuration::ResourceConfigurationAPI<DomainManagedInstanceId>>;
    fn node_registration_api(&mut self) -> Box<dyn crate::node_registration::NodeRegistrationAPI>;
}
