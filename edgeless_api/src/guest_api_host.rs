// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, PartialEq)]
pub struct OutputEventData {
    pub originator: edgeless_api_core::instance_id::InstanceId,
    pub alias: String,
    pub msg: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputEventDataRaw {
    pub originator: edgeless_api_core::instance_id::InstanceId,
    pub dst: edgeless_api_core::instance_id::InstanceId,
    pub msg: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TelemetryLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryLogEvent {
    pub originator: edgeless_api_core::instance_id::InstanceId,
    pub log_level: TelemetryLogLevel,
    pub target: String,
    pub msg: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DelayedEventData {
    pub originator: edgeless_api_core::instance_id::InstanceId,
    pub alias: String,
    pub msg: Vec<u8>,
    pub delay: u64,
    pub metadata: edgeless_api_core::event_metadata::EventMetadata,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncData {
    pub originator: edgeless_api_core::instance_id::InstanceId,
    pub serialized_data: Vec<u8>,
}

#[async_trait::async_trait]
pub trait GuestAPIHost: GuestAPIHostClone + Sync + Send {
    async fn cast(&mut self, event: OutputEventData) -> anyhow::Result<()>;
    async fn cast_raw(&mut self, event: OutputEventDataRaw) -> anyhow::Result<()>;
    async fn call(&mut self, event: OutputEventData) -> anyhow::Result<crate::guest_api_function::CallReturn>;
    async fn call_raw(&mut self, event: OutputEventDataRaw) -> anyhow::Result<crate::guest_api_function::CallReturn>;
    async fn telemetry_log(&mut self, event: TelemetryLogEvent) -> anyhow::Result<()>;
    async fn slf(&mut self) -> anyhow::Result<edgeless_api_core::instance_id::InstanceId>;
    async fn delayed_cast(&mut self, event: DelayedEventData) -> anyhow::Result<()>;
    async fn sync(&mut self, event: SyncData) -> anyhow::Result<()>;
}

// https://stackoverflow.com/a/30353928
pub trait GuestAPIHostClone {
    fn clone_box(&self) -> Box<dyn GuestAPIHost>;
}
impl<T> GuestAPIHostClone for T
where
    T: 'static + GuestAPIHost + Clone,
{
    fn clone_box(&self) -> Box<dyn GuestAPIHost> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn GuestAPIHost> {
    fn clone(&self) -> Box<dyn GuestAPIHost> {
        self.clone_box()
    }
}
