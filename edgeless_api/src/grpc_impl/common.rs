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
}
