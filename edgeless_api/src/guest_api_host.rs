// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionInstanceInit {
    pub init_payload: String,
    pub serialized_state: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputEventData {
    pub alias: String,
    pub msg: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputEventDataRaw {
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
    log_level: TelemetryLogLevel,
    target: String,
    msg: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DelayedEventData {
    alias: String,
    msg: Vec<u8>,
    delay: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncData {
    serialized_data: Vec<u8>,
}

#[async_trait::async_trait]
pub trait GuestAPIHostAPI: GuestAPIHostAPIClone + Sync + Send {
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
pub trait GuestAPIHostAPIClone {
    fn clone_box(&self) -> Box<dyn GuestAPIHostAPI>;
}
impl<T> GuestAPIHostAPIClone for T
where
    T: 'static + GuestAPIHostAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn GuestAPIHostAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn GuestAPIHostAPI> {
    fn clone(&self) -> Box<dyn GuestAPIHostAPI> {
        self.clone_box()
    }
}
