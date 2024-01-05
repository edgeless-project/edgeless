#[derive(Debug, Clone, PartialEq)]
pub struct ResourceProviderSpecification {
    pub provider_id: String,
    pub class_type: String,
    pub outputs: Vec<String>,
    pub configuration_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateNodeRequest {
    // 0: node_id (cannot be nil)
    // 1: agent_url (cannot be empty)
    // 2: invocation_url (cannot be empty)
    // 3: resource provider specifications (can be empty)
    Registration(uuid::Uuid, String, String, Vec<ResourceProviderSpecification>),

    // 0: node_id (cannot be empty)
    Deregistration(uuid::Uuid),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateNodeResponse {
    ResponseError(crate::common::ResponseError),
    Accepted,
}

#[async_trait::async_trait]
pub trait NodeRegistrationAPI: NodeRegistrationAPIClone + Sync + Send {
    async fn update_node(&mut self, request: UpdateNodeRequest) -> anyhow::Result<UpdateNodeResponse>;
}

impl std::fmt::Display for ResourceProviderSpecification {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "provider_id {}, class_type {}, outputs [{}], configuration_url {}",
            self.provider_id,
            self.class_type,
            self.outputs.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
            self.configuration_url
        )
    }
}

// https://stackoverflow.com/a/30353928
pub trait NodeRegistrationAPIClone {
    fn clone_box(&self) -> Box<dyn NodeRegistrationAPI>;
}
impl<T> NodeRegistrationAPIClone for T
where
    T: 'static + NodeRegistrationAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn NodeRegistrationAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn NodeRegistrationAPI> {
    fn clone(&self) -> Box<dyn NodeRegistrationAPI> {
        self.clone_box()
    }
}
