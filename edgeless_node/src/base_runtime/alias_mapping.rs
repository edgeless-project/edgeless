// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
/// Struct representing the updatable callbacks/aliases of a function instance.
/// Shared between a function instance's host and guest.
#[derive(Clone)]
pub struct AliasMapping {
    mapping: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, edgeless_api::function_instance::InstanceId>>>,
}

impl Default for AliasMapping {
    fn default() -> Self {
        Self::new()
    }
}

impl AliasMapping {
    pub fn new() -> Self {
        AliasMapping {
            mapping: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn get_mapping(&self, alias: &str) -> Option<edgeless_api::function_instance::InstanceId> {
        self.mapping.lock().await.get(alias).copied()
    }

    pub async fn update(&mut self, new_mapping: std::collections::HashMap<String, edgeless_api::function_instance::InstanceId>) {
        *self.mapping.lock().await = new_mapping;
    }
}
