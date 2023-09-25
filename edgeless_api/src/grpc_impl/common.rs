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
}
