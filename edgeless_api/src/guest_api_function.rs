// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, PartialEq)]
pub struct BootData {
    pub guest_api_host_endpoint: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionInstanceInit {
    pub init_payload: String,
    pub serialized_state: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputEventData {
    pub src: edgeless_api_core::instance_id::InstanceId,
    pub msg: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallReturn {
    NoRet,
    Reply(Vec<u8>),
    Err,
}

#[async_trait::async_trait]
pub trait GuestAPIFunction: GuestAPIFunctionClone + Sync + Send {
    async fn boot(&mut self, boot_data: BootData) -> anyhow::Result<()>;
    async fn init(&mut self, init_data: FunctionInstanceInit) -> anyhow::Result<()>;
    async fn cast(&mut self, event: InputEventData) -> anyhow::Result<()>;
    async fn call(&mut self, event: InputEventData) -> anyhow::Result<CallReturn>;
    async fn stop(&mut self) -> anyhow::Result<()>;
}

// https://stackoverflow.com/a/30353928
pub trait GuestAPIFunctionClone {
    fn clone_box(&self) -> Box<dyn GuestAPIFunction>;
}
impl<T> GuestAPIFunctionClone for T
where
    T: 'static + GuestAPIFunction + Clone,
{
    fn clone_box(&self) -> Box<dyn GuestAPIFunction> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn GuestAPIFunction> {
    fn clone(&self) -> Box<dyn GuestAPIFunction> {
        self.clone_box()
    }
}
