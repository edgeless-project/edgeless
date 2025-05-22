// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub enum EventData {
    Call(String),
    Cast(String),
    CallRet(String),
    CallNoRet,
    Err,
}

// custom display format
impl std::fmt::Display for EventData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EventData::Call(data) => write!(
                f,
                "Call (size {} bytes): {}",
                data.len(),
                core::str::from_utf8(data.as_bytes()).unwrap_or("not-utf-8")
            ),
            EventData::Cast(data) => write!(
                f,
                "Cast (size {} bytes): {}",
                data.len(),
                core::str::from_utf8(data.as_bytes()).unwrap_or("not-utf-8")
            ),
            EventData::CallRet(data) => write!(
                f,
                "CallRet (size {} bytes): {}",
                data.len(),
                core::str::from_utf8(data.as_bytes()).unwrap_or("not-utf-8")
            ),
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
    pub created: crate::function_instance::EventTimestamp,
    pub metadata: crate::function_instance::EventMetadata,
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
    // means that the event has been processed entirely and finally by the destination
    FINAL,
    // means that the event could not be handled by the link, so was ignored and
    // not delivered
    IGNORED,
    // means that some Dataplane level error has occured with a description
    ERROR(String),
}

#[async_trait::async_trait]
pub trait InvocationAPI: Sync + Send {
    // sends an event from source to target, non-blocking
    async fn handle(&mut self, event: Event) -> LinkProcessingResult;
}
