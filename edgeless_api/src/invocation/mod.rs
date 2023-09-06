#[derive(Clone)]
pub enum EventData {
    Call(String),
    Cast(String),
    CallRet(String),
    CallNoRet,
    Err,
}

#[derive(Clone)]
pub struct Event {
    pub target: crate::function_instance::FunctionId,
    pub source: crate::function_instance::FunctionId,
    pub stream_id: u64,
    pub data: EventData,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LinkProcessingResult {
    FINAL,
    PROCESSED,
    PASSED,
}

#[async_trait::async_trait]
pub trait InvocationAPI: Sync + Send {
    async fn handle(&mut self, event: Event) -> anyhow::Result<LinkProcessingResult>;
}
