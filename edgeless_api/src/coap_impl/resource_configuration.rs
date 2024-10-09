// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[async_trait::async_trait]
impl crate::resource_configuration::ResourceConfigurationAPI<edgeless_api_core::instance_id::InstanceId> for super::CoapClient {
    async fn start(
        &mut self,
        instance_specification: crate::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::common::StartComponentResponse<edgeless_api_core::instance_id::InstanceId>> {
        let mut outputs = heapless::Vec::<(&str, edgeless_api_core::instance_id::InstanceId), 16>::new();
        let mut configuration = heapless::Vec::<(&str, &str), 16>::new();
        for (key, val) in &instance_specification.output_mapping {
            outputs.push((key, *val)).map_err(|_| anyhow::anyhow!("Too many outputs"))?;
        }

        for (key, val) in &instance_specification.configuration {
            configuration
                .push((key, val))
                .map_err(|_| anyhow::anyhow!("Too many configuration options"))?;
        }

        let encoded_resource_spec = edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification {
            class_type: &instance_specification.class_type,
            output_mapping: outputs,
            configuration,
        };

        let res = self
            .call_with_reply(|token, addr, buffer| {
                edgeless_api_core::coap_mapping::COAPEncoder::encode_start_resource(addr, encoded_resource_spec.clone(), token, &mut buffer[..])
            })
            .await;

        match res {
            Ok(data) => Ok(crate::common::StartComponentResponse::InstanceId(
                edgeless_api_core::coap_mapping::CoapDecoder::decode_instance_id(&data).unwrap(),
            )),
            Err(data) => Ok(crate::common::StartComponentResponse::ResponseError(crate::common::ResponseError {
                summary: minicbor::decode::<&str>(&data).unwrap().to_string(),
                detail: None,
            })),
        }
    }

    async fn stop(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        let res = self
            .call_with_reply(|token, addr, buffer| {
                edgeless_api_core::coap_mapping::COAPEncoder::encode_stop_resource(addr, resource_id, token, &mut buffer[..])
            })
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(data) => Err(anyhow::anyhow!(core::str::from_utf8(&data).unwrap().to_string())),
        }
    }

    async fn patch(&mut self, update: crate::common::PatchRequest) -> anyhow::Result<()> {
        let mut outputs: [Option<(&str, edgeless_api_core::instance_id::InstanceId)>; 16] = [None; 16];
        let mut outputs_i: usize = 0;

        for (key, val) in &update.output_mapping {
            outputs[outputs_i] = Some((key, *val));
            outputs_i += 1;
        }

        let encoded_patch_req = edgeless_api_core::resource_configuration::EncodedPatchRequest {
            instance_id: edgeless_api_core::instance_id::InstanceId {
                node_id: edgeless_api_core::instance_id::NODE_ID_NONE,
                function_id: update.function_id,
            },
            output_mapping: outputs,
        };

        let res = self
            .call_with_reply(|token, addr, buffer| {
                edgeless_api_core::coap_mapping::COAPEncoder::encode_patch_request(addr, encoded_patch_req.clone(), token, &mut buffer[..])
            })
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(data) => Err(anyhow::anyhow!(core::str::from_utf8(&data).unwrap().to_string())),
        }
    }
}
