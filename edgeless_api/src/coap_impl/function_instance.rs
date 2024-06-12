// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[async_trait::async_trait]
impl crate::function_instance::FunctionInstanceAPI<edgeless_api_core::instance_id::InstanceId> for super::CoapClient {
    async fn start(
        &mut self,
        _spawn_request: crate::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<crate::common::StartComponentResponse<edgeless_api_core::instance_id::InstanceId>> {
        todo!()
    }
    async fn stop(&mut self, _id: edgeless_api_core::instance_id::InstanceId) -> anyhow::Result<()> {
        todo!()
    }
    async fn patch(&mut self, _update: crate::common::PatchRequest) -> anyhow::Result<()> {
        todo!()
    }
}
