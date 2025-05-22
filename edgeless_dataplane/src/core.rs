// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessDataplanePeerSettings {
    pub node_id: uuid::Uuid,
    pub invocation_url: String,
}
/// Trait that needs to be implemented by each link that is added to a dataplane chain.
/// Link instances are commonly created by a LinkProvider.
/// All communication on the dataplane is unreliable and non-blocking and using "casts". Higher
/// level mechanisms can be implemented by specific implementations.
#[async_trait::async_trait]
pub trait DataPlaneLink: Send + Sync {
    async fn handle_cast(
        &mut self,
        target: &edgeless_api::function_instance::InstanceId,
        msg: Message,
        src: &edgeless_api::function_instance::InstanceId,
        created: &edgeless_api::function_instance::EventTimestamp,
        channel_id: u64,
        metadata: &edgeless_api::function_instance::EventMetadata,
    ) -> LinkProcessingResult;
}

// TODO: clean this up - it is already defined as a message
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallRet {
    NoReply,
    Reply(String),
    // Error can be anything that happens on the application level of the dataplane
    Err(String),
}

#[derive(Clone, Debug)]
pub struct DataplaneEvent {
    pub source_id: edgeless_api::function_instance::InstanceId,
    pub channel_id: u64,
    pub message: Message,
    pub created: edgeless_api::function_instance::EventTimestamp,
    pub metadata: edgeless_api::function_instance::EventMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    Cast(String),
    Call(String),
    CallRet(String),
    CallNoRet,
    Err(String),
}
