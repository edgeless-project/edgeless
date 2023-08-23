pub use edgeless_api::invocation::LinkProcessingResult;

/// Trait that needs to be implemented by each link that is added to a dataplane chain.
/// Link instances are commonly created by a LinkProvider (which is not a trait yet).
#[async_trait::async_trait]
pub trait DataPlaneLink: Send + Sync {
    async fn handle_send(
        &mut self,
        target: &edgeless_api::function_instance::FunctionId,
        msg: Message,
        src: &edgeless_api::function_instance::FunctionId,
        channel_id: u64,
    ) -> LinkProcessingResult;
}

#[derive(Clone, Debug)]
pub enum CallRet {
    NoReply,
    Reply(String),
    Err,
}

#[derive(Clone, Debug)]
pub enum Message {
    Cast(String),
    Call(String),
    CallRet(String),
    CallNoRet,
    Err,
}

#[derive(Clone, Debug)]
pub struct DataplaneEvent {
    pub source_id: edgeless_api::function_instance::FunctionId,
    pub channel_id: u64,
    pub message: Message,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EdgelessDataplanePeerSettings {
    pub id: uuid::Uuid,
    pub invocation_url: String,
}
