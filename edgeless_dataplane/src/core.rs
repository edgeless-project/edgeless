// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
pub use edgeless_api::invocation::LinkProcessingResult;

/// Trait that needs to be implemented by each link that is added to a dataplane chain.
/// Link instances are commonly created by a LinkProvider (which is not a trait yet).
#[async_trait::async_trait]
pub trait DataPlaneLink: Send + Sync {
    async fn handle_send(
        &mut self,
        target: &edgeless_api::function_instance::InstanceId,
        msg: Message,
        src: &edgeless_api::function_instance::InstanceId,
        created: &edgeless_api::function_instance::EventTimestamp,
        channel_id: u64,
        metadata: &edgeless_api::function_instance::EventMetadata,
    ) -> LinkProcessingResult;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallRet {
    NoReply,
    Reply(String),
    Err,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    Cast(String),
    Call(String),
    CallRet(String),
    CallNoRet,
    Err,
}

#[derive(Clone, Debug)]
pub struct DataplaneEvent {
    pub source_id: edgeless_api::function_instance::InstanceId,
    pub channel_id: u64,
    pub message: Message,
    pub created: edgeless_api::function_instance::EventTimestamp,
    pub metadata: edgeless_api::function_instance::EventMetadata,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessDataplanePeerSettings {
    pub node_id: uuid::Uuid,
    pub invocation_url: String,
}
