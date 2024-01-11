// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#[derive(Clone)]
pub struct StateManager {
    handlers: std::sync::Arc<tokio::sync::Mutex<StateProviders>>,
}

#[async_trait::async_trait]
trait StateProvider: Sync + Send {
    fn get(&mut self, state_id: uuid::Uuid) -> Option<String>;
    fn set(&mut self, state_id: uuid::Uuid, serialized_state: String);
}

struct FileStateProvider {
    base_path: std::path::PathBuf,
}

impl FileStateProvider {
    fn new() -> Self {
        std::fs::DirBuilder::new().recursive(true).create("./function_state/").unwrap();
        Self {
            base_path: std::path::PathBuf::from("./function_state/"),
        }
    }
}

impl StateProvider for FileStateProvider {
    fn get(&mut self, state_id: uuid::Uuid) -> Option<String> {
        let state_file = self.base_path.join(state_id.to_string());
        if state_file.exists() {
            return Some(std::fs::read_to_string(state_file).unwrap());
        }
        None
    }
    fn set(&mut self, state_id: uuid::Uuid, serialized_state: String) {
        let state_file = self.base_path.join(state_id.to_string());
        std::fs::write(state_file, serialized_state).unwrap();
    }
}

struct StateProviders {
    node_local: Option<Box<dyn StateProvider>>,
    global: Option<Box<dyn StateProvider>>,
}

#[async_trait::async_trait]
pub trait StateManagerAPI: Send {
    async fn get_handle(&mut self, state_policy: edgeless_api::function_instance::StatePolicy, state_id: uuid::Uuid) -> Box<dyn StateHandleAPI>;
}

impl StateManager {
    pub async fn new() -> Self {
        Self {
            handlers: std::sync::Arc::new(tokio::sync::Mutex::new(StateProviders {
                node_local: Some(Box::new(FileStateProvider::new())),
                global: None,
            })),
        }
    }
}

#[async_trait::async_trait]
impl StateManagerAPI for StateManager {
    async fn get_handle(&mut self, state_policy: edgeless_api::function_instance::StatePolicy, state_id: uuid::Uuid) -> Box<dyn StateHandleAPI> {
        Box::new(StateHandle {
            state_policy,
            state_id,
            handlers: self.handlers.clone(),
        })
    }
}

#[async_trait::async_trait]
pub trait StateHandleAPI: Send {
    async fn get(&mut self) -> Option<String>;
    async fn set(&mut self, serialized_state: String);
}

pub struct StateHandle {
    handlers: std::sync::Arc<tokio::sync::Mutex<StateProviders>>,
    state_id: uuid::Uuid,
    state_policy: edgeless_api::function_instance::StatePolicy,
}

#[async_trait::async_trait]
impl StateHandleAPI for StateHandle {
    async fn get(&mut self) -> Option<String> {
        let mut handles = self.handlers.lock().await;
        match self.state_policy {
            edgeless_api::function_instance::StatePolicy::NodeLocal => {
                if let Some(provider) = &mut handles.node_local {
                    return provider.get(self.state_id);
                }
            }
            edgeless_api::function_instance::StatePolicy::Global => {
                if let Some(provider) = &mut handles.global {
                    return provider.get(self.state_id);
                }
            }
            _ => {}
        }
        None
    }

    async fn set(&mut self, serialized_state: String) {
        let mut handles = self.handlers.lock().await;
        match self.state_policy {
            edgeless_api::function_instance::StatePolicy::NodeLocal => {
                if let Some(provider) = &mut handles.node_local {
                    return provider.set(self.state_id, serialized_state);
                }
            }
            edgeless_api::function_instance::StatePolicy::Global => {
                if let Some(provider) = &mut handles.global {
                    return provider.set(self.state_id, serialized_state);
                }
            }
            _ => {}
        }
    }
}
