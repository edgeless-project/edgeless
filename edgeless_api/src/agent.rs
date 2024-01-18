// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
pub trait AgentAPI {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceAPI<edgeless_api_core::instance_id::InstanceId>>;
    fn node_management_api(&mut self) -> Box<dyn crate::node_management::NodeManagementAPI>;
    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn crate::resource_configuration::ResourceConfigurationAPI<edgeless_api_core::instance_id::InstanceId>>;
}
