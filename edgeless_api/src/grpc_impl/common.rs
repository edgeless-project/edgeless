// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
pub struct CommonConverters {}

pub trait ParseableId<IdType> {
    fn parse(api_id_variant: &Self) -> anyhow::Result<IdType>;
}
pub trait SerializeableId {
    fn serialize(id: &Self) -> crate::grpc_impl::api::InstanceIdVariant;
}

impl ParseableId<edgeless_api_core::instance_id::InstanceId> for crate::grpc_impl::api::InstanceIdVariant {
    fn parse(api_id_variant: &Self) -> anyhow::Result<crate::function_instance::InstanceId> {
        match api_id_variant.clone().instance_id_type.ok_or(anyhow::anyhow!("Missing Id"))? {
            crate::grpc_impl::api::instance_id_variant::InstanceIdType::InstanceId(instance_id) => CommonConverters::parse_instance_id(&instance_id),
            _ => Err(anyhow::anyhow!("Wrong Type")),
        }
    }
}

impl ParseableId<crate::orc::DomainManagedInstanceId> for crate::grpc_impl::api::InstanceIdVariant {
    fn parse(api_id_variant: &Self) -> anyhow::Result<crate::orc::DomainManagedInstanceId> {
        match api_id_variant.clone().instance_id_type.ok_or(anyhow::anyhow!("Missing Id"))? {
            crate::grpc_impl::api::instance_id_variant::InstanceIdType::DomainManagedInstanceId(instance_id) => {
                CommonConverters::parse_domain_managed_instance_id(&instance_id)
            }
            _ => Err(anyhow::anyhow!("Wrong Type")),
        }
    }
}

impl SerializeableId for edgeless_api_core::instance_id::InstanceId {
    fn serialize(id: &Self) -> crate::grpc_impl::api::InstanceIdVariant {
        crate::grpc_impl::api::InstanceIdVariant {
            instance_id_type: Some(crate::grpc_impl::api::instance_id_variant::InstanceIdType::InstanceId(
                CommonConverters::serialize_instance_id(id),
            )),
        }
    }
}

impl SerializeableId for crate::orc::DomainManagedInstanceId {
    fn serialize(id: &Self) -> crate::grpc_impl::api::InstanceIdVariant {
        crate::grpc_impl::api::InstanceIdVariant {
            instance_id_type: Some(crate::grpc_impl::api::instance_id_variant::InstanceIdType::DomainManagedInstanceId(
                CommonConverters::serialize_domain_managed_instance_id(id),
            )),
        }
    }
}

impl CommonConverters {
    pub fn parse_response_error(api_request: &crate::grpc_impl::api::ResponseError) -> anyhow::Result<crate::common::ResponseError> {
        Ok(crate::common::ResponseError {
            summary: api_request.summary.to_string(),
            detail: api_request.detail.clone(),
        })
    }

    pub fn parse_output(api_output: &crate::grpc_impl::api::InstanceOutput) -> anyhow::Result<crate::common::Output> {
        Ok(match api_output.output_type.as_ref().unwrap() {
            crate::grpc_impl::api::instance_output::OutputType::Single(target) => {
                let (instance, port) = Self::parse_target(target)?;
                crate::common::Output::Single(instance, port)
            }
            crate::grpc_impl::api::instance_output::OutputType::Any(targets) => {
                crate::common::Output::Any(targets.data.iter().map(|target| Self::parse_target(target).unwrap()).collect())
            }
            crate::grpc_impl::api::instance_output::OutputType::All(targets) => {
                crate::common::Output::All(targets.data.iter().map(|target| Self::parse_target(target).unwrap()).collect())
            }
            crate::grpc_impl::api::instance_output::OutputType::Link(link_id) => crate::common::Output::Link(Self::parse_link_id(link_id)?),
        })
    }

    pub fn parse_input(api_input: &crate::grpc_impl::api::InstanceInput) -> anyhow::Result<crate::common::Input> {
        Ok(match api_input.input_type.as_ref().unwrap() {
            crate::grpc_impl::api::instance_input::InputType::Link(link_id) => crate::common::Input::Link(Self::parse_link_id(link_id)?),
        })
    }

    pub fn parse_target(
        api_target: &crate::grpc_impl::api::Target,
    ) -> anyhow::Result<(crate::function_instance::InstanceId, crate::function_instance::PortId)> {
        Ok((
            Self::parse_instance_id(api_target.instance_id.as_ref().unwrap())?,
            crate::function_instance::PortId(api_target.port_id.clone()),
        ))
    }

    pub fn parse_instance_id(api_id: &crate::grpc_impl::api::InstanceId) -> anyhow::Result<crate::function_instance::InstanceId> {
        Ok(crate::function_instance::InstanceId {
            node_id: uuid::Uuid::parse_str(&api_id.node_id)?,
            function_id: uuid::Uuid::parse_str(&api_id.function_id)?,
        })
    }

    pub fn parse_link_id(api_id: &crate::grpc_impl::api::LinkInstanceId) -> anyhow::Result<crate::link::LinkInstanceId> {
        Ok(crate::link::LinkInstanceId(uuid::Uuid::parse_str(&api_id.id)?))
    }

    pub fn parse_domain_managed_instance_id(
        api_id: &crate::grpc_impl::api::DomainManagedInstanceId,
    ) -> anyhow::Result<crate::orc::DomainManagedInstanceId> {
        Ok(uuid::Uuid::parse_str(&api_id.instance_id)?)
    }

    pub fn parse_start_component_response<ResourceIdType>(
        api_instance: &crate::grpc_impl::api::StartComponentResponse,
    ) -> anyhow::Result<crate::common::StartComponentResponse<ResourceIdType>>
    where
        super::api::InstanceIdVariant: ParseableId<ResourceIdType>,
    {
        match api_instance.instance_id.as_ref() {
            Some(val) => match ParseableId::<ResourceIdType>::parse(val) {
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
            function_id: super::common::CommonConverters::parse_instance_id(api_update.function_id.as_ref().unwrap())?,
            output_mapping: api_update
                .output_mapping
                .iter()
                .filter_map(|(key, value)| match CommonConverters::parse_output(value) {
                    Ok(val) => Some((crate::function_instance::PortId(key.clone()), val)),
                    Err(_) => None,
                })
                .collect(),
            input_mapping: std::collections::HashMap::new(),
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

    pub fn serialize_link_instance_id(instance_id: &crate::link::LinkInstanceId) -> crate::grpc_impl::api::LinkInstanceId {
        crate::grpc_impl::api::LinkInstanceId {
            id: instance_id.0.to_string(),
        }
    }

    pub fn serialize_domain_managed_instance_id(instance_id: &crate::orc::DomainManagedInstanceId) -> crate::grpc_impl::api::DomainManagedInstanceId {
        crate::grpc_impl::api::DomainManagedInstanceId {
            instance_id: instance_id.to_string(),
        }
    }

    pub fn serialize_start_component_response<ComponentIdType: SerializeableId>(
        req: &crate::common::StartComponentResponse<ComponentIdType>,
    ) -> crate::grpc_impl::api::StartComponentResponse {
        match req {
            crate::common::StartComponentResponse::ResponseError(err) => crate::grpc_impl::api::StartComponentResponse {
                response_error: Some(CommonConverters::serialize_response_error(err)),
                instance_id: None,
            },
            crate::common::StartComponentResponse::InstanceId(id) => crate::grpc_impl::api::StartComponentResponse {
                response_error: None,
                instance_id: Some(SerializeableId::serialize(id)),
            },
        }
    }

    pub fn serialize_patch_request(crate_update: &crate::common::PatchRequest) -> crate::grpc_impl::api::PatchRequest {
        crate::grpc_impl::api::PatchRequest {
            function_id: Some(super::common::CommonConverters::serialize_instance_id(&crate_update.function_id)),
            output_mapping: crate_update
                .output_mapping
                .iter()
                .map(|(key, value)| (key.0.clone(), Self::serialize_output(value)))
                .collect(),
            input_mapping: std::collections::HashMap::new(),
        }
    }

    pub fn serialize_output(crate_output: &crate::common::Output) -> super::api::InstanceOutput {
        match crate_output {
            crate::common::Output::Single(instance_id, port_id) => super::api::InstanceOutput {
                output_type: Some(super::api::instance_output::OutputType::Single(Self::serialize_target(
                    instance_id,
                    port_id,
                ))),
            },
            crate::common::Output::Any(targets) => super::api::InstanceOutput {
                output_type: Some(super::api::instance_output::OutputType::Any(super::api::TargetVec {
                    data: targets
                        .iter()
                        .map(|(instance_id, port_id)| CommonConverters::serialize_target(instance_id, port_id))
                        .collect(),
                })),
            },
            crate::common::Output::All(targets) => super::api::InstanceOutput {
                output_type: Some(super::api::instance_output::OutputType::All(super::api::TargetVec {
                    data: targets
                        .iter()
                        .map(|(instance_id, port_id)| CommonConverters::serialize_target(instance_id, port_id))
                        .collect(),
                })),
            },
            crate::common::Output::Link(link_instance_id) => super::api::InstanceOutput {
                output_type: Some(super::api::instance_output::OutputType::Link(
                    CommonConverters::serialize_link_instance_id(link_instance_id),
                )),
            },
        }
    }

    pub fn serialize_input(crate_input: &crate::common::Input) -> super::api::InstanceInput {
        match crate_input {
            crate::common::Input::Link(link_instance_id) => super::api::InstanceInput {
                input_type: Some(super::api::instance_input::InputType::Link(CommonConverters::serialize_link_instance_id(
                    link_instance_id,
                ))),
            },
        }
    }

    pub fn serialize_target(instance_id: &crate::function_instance::InstanceId, port_id: &crate::function_instance::PortId) -> super::api::Target {
        super::api::Target {
            instance_id: Some(CommonConverters::serialize_instance_id(instance_id)),
            port_id: port_id.0.clone(),
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
                        crate::common::Output::Single(
                            InstanceId {
                                node_id: uuid::Uuid::new_v4(),
                                function_id: uuid::Uuid::new_v4(),
                            },
                            crate::function_instance::PortId("test".to_string()),
                        ),
                    ),
                    (
                        "err".to_string(),
                        crate::common::Output::Single(
                            InstanceId {
                                node_id: uuid::Uuid::new_v4(),
                                function_id: uuid::Uuid::new_v4(),
                            },
                            crate::function_instance::PortId("test".to_string()),
                        ),
                    ),
                ]),
            },
            PatchRequest {
                function_id: uuid::Uuid::new_v4(),
                output_mapping: std::collections::HashMap::from([
                    (
                        "out".to_string(),
                        crate::common::Output::Single(
                            InstanceId {
                                node_id: uuid::Uuid::nil(),
                                function_id: uuid::Uuid::new_v4(),
                            },
                            crate::function_instance::PortId("test".to_string()),
                        ),
                    ),
                    (
                        "err".to_string(),
                        crate::common::Output::Single(
                            InstanceId {
                                node_id: uuid::Uuid::nil(),
                                function_id: uuid::Uuid::new_v4(),
                            },
                            crate::function_instance::PortId("test".to_string()),
                        ),
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
