#[derive(Clone)]
pub struct StateManager {
    handlers: std::sync::Arc<tokio::sync::Mutex<StateProviders>>
}

#[async_trait::async_trait]
trait StateProvider : Sync + Send {
    fn get(&mut self, state_id: uuid::Uuid) -> Option<String>;
    fn set(&mut self, state_id: uuid::Uuid, serialized_state: String);
}

struct FileStateProvider {
    base_path: std::path::PathBuf
}

impl FileStateProvider {
    fn new() -> Self {
        if !std::path::Path::new("./function_state/").exists() {
            std::fs::DirBuilder::new().create("./function_state/").unwrap();
        }
        Self {
            base_path: std::path::PathBuf::from("./function_state/")
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
    global: Option<Box<dyn StateProvider>>
}


impl StateManager {

    pub async fn new() -> Self {
        Self {
            handlers:  std::sync::Arc::new(
                tokio::sync::Mutex::new(
                    StateProviders {
                        node_local: Some(Box::new(FileStateProvider::new())),
                        global: None
                    }
                )
            )
        }
    }

    pub async fn get_handle(&mut self, state_policy: edgeless_api::function_instance::StatePolicy, state_id: uuid::Uuid) -> StateHandle {
        StateHandle {
            state_policy,
            state_id,
            handlers: self.handlers.clone()
        }
    }
}

pub struct StateHandle {
    handlers: std::sync::Arc<tokio::sync::Mutex<StateProviders>>,
    state_id: uuid::Uuid,
    state_policy: edgeless_api::function_instance::StatePolicy
}

impl StateHandle {
    pub async fn get(&mut self) -> Option<String> {
        let mut handles = self.handlers.lock().await;
        match self.state_policy {
            edgeless_api::function_instance::StatePolicy::NodeLocal => {
                if let Some(provider) = &mut handles.node_local {
                    return provider.get(self.state_id);
                }
            }
            _ => {

            }
        }
        None
    }

    pub async fn set(&mut self, serialized_state: String) {
        let mut handles = self.handlers.lock().await;
        match self.state_policy {
            edgeless_api::function_instance::StatePolicy::NodeLocal => {
                if let Some(provider) = &mut handles.node_local {
                    return provider.set(self.state_id, serialized_state);
                }
            }
            _ => {

            }
        }
    }
}