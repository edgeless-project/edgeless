// SPDX-FileCopyrightText: © 2024 TUM
// SPDX-License-Identifier: MIT
pub mod alias_mapping;
pub mod function_instance_runner;
pub mod guest_api;
pub mod runtime;

/// (Deprecated) Trait to be implemented by each runtime.
/// You won't need to implement this trait if you use the base_runtime that is generic over the `FunctionInstance` trait
#[async_trait::async_trait]
pub trait RuntimeAPI {
    async fn start(&mut self, request: edgeless_api::function_instance::SpawnFunctionRequest) -> anyhow::Result<()>;
    async fn stop(&mut self, instance_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()>;
    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()>;
}

/// This must be implemented for each virtualization technology.
/// As suggested by the name, it contains a single instance of a function.
#[async_trait::async_trait]
pub trait FunctionInstance: Send + 'static {
    async fn instantiate(guest_api_host: crate::base_runtime::guest_api::GuestAPIHost, code: &[u8]) -> Result<Box<Self>, FunctionInstanceError>;
    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&str>) -> Result<(), FunctionInstanceError>;
    async fn cast(&mut self, src: &edgeless_api::function_instance::InstanceId, msg: &str) -> Result<(), FunctionInstanceError>;
    async fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        msg: &str,
    ) -> Result<edgeless_dataplane::core::CallRet, FunctionInstanceError>;
    async fn stop(&mut self) -> Result<(), FunctionInstanceError>;
}

#[derive(Clone, Debug)]
pub enum FunctionInstanceError {
    BadCode,
    InternalError,
}
