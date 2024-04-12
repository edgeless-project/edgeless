// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// FunctionInstance implementation allowing to execute functions defined
/// as computational containers through a gRPC API.
pub struct ContainerFunctionInstance {}

#[async_trait::async_trait]
impl crate::base_runtime::FunctionInstance for ContainerFunctionInstance {
    async fn instantiate(
        guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
        code: &[u8],
    ) -> Result<Box<Self>, crate::base_runtime::FunctionInstanceError> {
        log::info!("container run-time: instantiate {}", String::from_utf8(code.to_vec()).unwrap_or_default());
        Ok(Box::new(Self {}))
    }

    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&str>) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::debug!(
            "container run-time: init, payload {}, serialized_state {} bytes",
            init_payload.unwrap_or_default(),
            serialized_state.unwrap_or_default().len()
        );
        Ok(())
    }

    async fn cast(&mut self, src: &edgeless_api::function_instance::InstanceId, msg: &str) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::debug!("container run-time: cast, src {}, msg {} bytes", src, msg.len());
        Ok(())
    }

    async fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        msg: &str,
    ) -> Result<edgeless_dataplane::core::CallRet, crate::base_runtime::FunctionInstanceError> {
        log::debug!("container run-time: call, src {}, msg {} bytes", src, msg.len());
        Ok(edgeless_dataplane::core::CallRet::NoReply)
    }

    async fn stop(&mut self) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::debug!("container run-time: stop");
        Ok(())
    }
}
