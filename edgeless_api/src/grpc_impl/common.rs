pub struct CommonConverters {}

impl CommonConverters {
    pub fn parse_response_error(api_request: &crate::grpc_impl::api::ResponseError) -> anyhow::Result<crate::common::ResponseError> {
        Ok(crate::common::ResponseError {
            summary: api_request.summary.to_string(),
            detail: api_request.detail.clone(),
        })
    }

    pub fn parse_instance_id(api_id: &crate::grpc_impl::api::InstanceId) -> anyhow::Result<crate::function_instance::InstanceId> {
        Ok(crate::function_instance::InstanceId {
            node_id: uuid::Uuid::parse_str(&api_id.node_id)?,
            function_id: uuid::Uuid::parse_str(&api_id.function_id)?,
        })
    }

    pub fn parse_start_component_response(
        api_instance: &crate::grpc_impl::api::StartComponentResponse,
    ) -> anyhow::Result<crate::common::StartComponentResponse> {
        match api_instance.instance_id.as_ref() {
            Some(val) => match CommonConverters::parse_instance_id(val) {
                Ok(val) => Ok(crate::common::StartComponentResponse::InstanceId(val)),
                Err(err) => Err(anyhow::anyhow!(err.to_string())),
            },
            None => match api_instance.response_error.as_ref() {
                Some(val) => match CommonConverters::parse_response_error(val) {
                    Ok(val) => Ok(crate::common::StartComponentResponse::ResponseError(val)),
                    Err(err) => Err(anyhow::anyhow!(err.to_string())),
                },
                None => Err(anyhow::anyhow!(
                    "Ill-formed StartComponentResponse message: both ResponseError and InstanceId are empty"
                )),
            },
        }
    }

    pub fn parse_patch_request(api_update: &crate::grpc_impl::api::PatchRequest) -> anyhow::Result<crate::common::PatchRequest> {
        Ok(crate::common::PatchRequest {
            function_id: uuid::Uuid::parse_str(&api_update.function_id)?,
            output_mapping: api_update
                .output_mapping
                .iter()
                .filter_map(|(key, value)| {
                    return {
                        match CommonConverters::parse_instance_id(&value) {
                            Ok(val) => Some((key.clone(), val)),
                            Err(_) => None,
                        }
                    };
                })
                .collect(),
        })
    }

    pub fn serialize_response_error(crate_function: &crate::common::ResponseError) -> crate::grpc_impl::api::ResponseError {
        crate::grpc_impl::api::ResponseError {
            summary: crate_function.summary.clone(),
            detail: crate_function.detail.clone(),
        }
    }

    pub fn serialize_instance_id(instance_id: &crate::function_instance::InstanceId) -> crate::grpc_impl::api::InstanceId {
        crate::grpc_impl::api::InstanceId {
            node_id: instance_id.node_id.to_string(),
            function_id: instance_id.function_id.to_string(),
        }
    }

    pub fn serialize_start_component_response(req: &crate::common::StartComponentResponse) -> crate::grpc_impl::api::StartComponentResponse {
        match req {
            crate::common::StartComponentResponse::ResponseError(err) => crate::grpc_impl::api::StartComponentResponse {
                response_error: Some(CommonConverters::serialize_response_error(&err)),
                instance_id: None,
            },
            crate::common::StartComponentResponse::InstanceId(id) => crate::grpc_impl::api::StartComponentResponse {
                response_error: None,
                instance_id: Some(CommonConverters::serialize_instance_id(&id)),
            },
        }
    }

    pub fn serialize_patch_request(crate_update: &crate::common::PatchRequest) -> crate::grpc_impl::api::PatchRequest {
        crate::grpc_impl::api::PatchRequest {
            function_id: crate_update.function_id.to_string(),
            output_mapping: crate_update
                .output_mapping
                .iter()
                .map(|(key, value)| (key.clone(), CommonConverters::serialize_instance_id(&value)))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use edgeless_api_core::instance_id::InstanceId;

    use super::*;
    use crate::common::PatchRequest;

    #[test]
    fn serialize_deserialize_patch_request() {
        let messages = vec![
            PatchRequest {
                function_id: uuid::Uuid::new_v4(),
                output_mapping: std::collections::HashMap::from([
                    (
                        "out".to_string(),
                        InstanceId {
                            node_id: uuid::Uuid::new_v4(),
                            function_id: uuid::Uuid::new_v4(),
                        },
                    ),
                    (
                        "err".to_string(),
                        InstanceId {
                            node_id: uuid::Uuid::new_v4(),
                            function_id: uuid::Uuid::new_v4(),
                        },
                    ),
                ]),
            },
            PatchRequest {
                function_id: uuid::Uuid::new_v4(),
                output_mapping: std::collections::HashMap::from([
                    (
                        "out".to_string(),
                        InstanceId {
                            node_id: uuid::Uuid::nil(),
                            function_id: uuid::Uuid::new_v4(),
                        },
                    ),
                    (
                        "err".to_string(),
                        InstanceId {
                            node_id: uuid::Uuid::nil(),
                            function_id: uuid::Uuid::new_v4(),
                        },
                    ),
                ]),
            },
        ];
        for msg in messages {
            match CommonConverters::parse_patch_request(&CommonConverters::serialize_patch_request(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }
}
