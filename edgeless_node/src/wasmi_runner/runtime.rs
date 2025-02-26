// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub struct WasmiRuntime {
    _configuration: std::collections::HashMap<String, String>,
}

impl WasmiRuntime {
    pub fn new() -> Self {
        Self {
            _configuration: std::collections::HashMap::new(),
        }
    }
}

impl Default for WasmiRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::base_runtime::runtime::GuestAPIHostRegister for WasmiRuntime {
    fn needs_to_register(&mut self) -> bool {
        false
    }
    fn register_guest_api_host(
        &mut self,
        _instance_id: &edgeless_api::function_instance::InstanceId,
        _guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
    ) {
    }

    fn deregister_guest_api_host(&mut self, _instance_id: &edgeless_api::function_instance::InstanceId) {}

    fn guest_api_host(
        &mut self,
        _instance_id: &edgeless_api::function_instance::InstanceId,
    ) -> Option<&mut crate::base_runtime::guest_api::GuestAPIHost> {
        None
    }

    fn configuration(&mut self) -> std::collections::HashMap<String, String> {
        self._configuration.clone()
    }
}
