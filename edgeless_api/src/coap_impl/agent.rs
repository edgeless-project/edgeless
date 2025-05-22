// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

impl crate::outer::agent::AgentAPI for super::CoapClient {
    fn function_instance_api(&mut self) -> Box<dyn crate::function_instance::FunctionInstanceAPI<edgeless_api_core::instance_id::InstanceId>> {
        crate::function_instance::FunctionInstanceAPIClone::clone_box(self)
    }

    fn node_management_api(&mut self) -> Box<dyn crate::node_management::NodeManagementAPI> {
        crate::node_management::NodeManagementAPIClone::clone_box(self)
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn crate::resource_configuration::ResourceConfigurationAPI<edgeless_api_core::instance_id::InstanceId>> {
        crate::resource_configuration::ResourceConfigurationAPIClone::clone_box(self)
    }
}
