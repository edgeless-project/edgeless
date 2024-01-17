// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub enum EventData {
    Call(String),
    Cast(String),
    CallRet(String),
    CallNoRet,
    Err,
}

impl std::fmt::Display for EventData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EventData::Call(data) => write!(f, "Call (size {} bytes)", data.len()),
            EventData::Cast(data) => write!(f, "Call (size {} bytes)", data.len()),
            EventData::CallRet(data) => write!(f, "Call (size {} bytes)", data.len()),
            EventData::CallNoRet => write!(f, "CallNoRet"),
            EventData::Err => write!(f, "Err"),
        }
    }
}

#[derive(Clone)]
pub struct Event {
    pub target: crate::function_instance::InstanceId,
    pub source: crate::function_instance::InstanceId,
    pub stream_id: u64,
    pub data: EventData,
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "target node {} component {}, source node {} component {}, stream_id {}, {}",
            self.target.node_id, self.target.function_id, self.source.node_id, self.source.function_id, self.stream_id, self.data
        )
    }
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
