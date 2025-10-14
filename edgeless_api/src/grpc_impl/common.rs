// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
pub struct CommonConverters {}

pub trait ParseableId<IdType> {
    fn parse(api_id_variant: &Self) -> anyhow::Result<IdType>;
}
pub trait SerializeableId {
    fn serialize(id: &Self) -> crate::grpc_impl::api::InstanceIdVariant;
}

impl ParseableId<edgeless_api_core::instance_id::InstanceId>
    for crate::grpc_impl::api::InstanceIdVariant
{
    fn parse(api_id_variant: &Self) -> anyhow::Result<crate::function_instance::InstanceId> {
        match api_id_variant
            .clone()
            .instance_id_type
            .ok_or(anyhow::anyhow!("Missing Id"))?
        {
            crate::grpc_impl::api::instance_id_variant::InstanceIdType::InstanceId(instance_id) => {
                CommonConverters::parse_instance_id(&instance_id)
            }
            _ => Err(anyhow::anyhow!("Wrong Type")),
        }
    }
}

impl ParseableId<crate::function_instance::DomainManagedInstanceId>
    for crate::grpc_impl::api::InstanceIdVariant
{
    fn parse(
        api_id_variant: &Self,
    ) -> anyhow::Result<crate::function_instance::DomainManagedInstanceId> {
        match api_id_variant
            .clone()
            .instance_id_type
            .ok_or(anyhow::anyhow!("Missing Id"))?
        {
            crate::grpc_impl::api::instance_id_variant::InstanceIdType::DomainManagedInstanceId(
                instance_id,
            ) => CommonConverters::parse_domain_managed_instance_id(&instance_id),
            _ => Err(anyhow::anyhow!("Wrong Type")),
        }
    }
}

impl SerializeableId for edgeless_api_core::instance_id::InstanceId {
    fn serialize(id: &Self) -> crate::grpc_impl::api::InstanceIdVariant {
        crate::grpc_impl::api::InstanceIdVariant {
            instance_id_type: Some(
                crate::grpc_impl::api::instance_id_variant::InstanceIdType::InstanceId(
                    CommonConverters::serialize_instance_id(id),
                ),
            ),
        }
    }
}

impl SerializeableId for crate::function_instance::DomainManagedInstanceId {
    fn serialize(id: &Self) -> crate::grpc_impl::api::InstanceIdVariant {
        crate::grpc_impl::api::InstanceIdVariant {
            instance_id_type: Some(
                crate::grpc_impl::api::instance_id_variant::InstanceIdType::DomainManagedInstanceId(
                    CommonConverters::serialize_domain_managed_instance_id(id),
                ),
            ),
        }
    }
}

impl CommonConverters {
    pub fn parse_response_error(
        api_request: &crate::grpc_impl::api::ResponseError,
    ) -> anyhow::Result<crate::common::ResponseError> {
        Ok(crate::common::ResponseError {
            summary: api_request.summary.to_string(),
            detail: api_request.detail.clone(),
        })
    }

    pub fn parse_instance_id(
        api_id: &crate::grpc_impl::api::InstanceId,
    ) -> anyhow::Result<crate::function_instance::InstanceId> {
        Ok(crate::function_instance::InstanceId {
            node_id: uuid::Uuid::parse_str(&api_id.node_id)?,
            function_id: uuid::Uuid::parse_str(&api_id.function_id)?,
        })
    }

    pub fn parse_event_timestamp(
        api_ts: &crate::grpc_impl::api::EventTimestamp,
    ) -> anyhow::Result<crate::function_instance::EventTimestamp> {
        Ok(crate::function_instance::EventTimestamp {
            secs: api_ts.secs,
            nsecs: api_ts.nsecs,
        })
    }

    pub fn parse_domain_managed_instance_id(
        api_id: &crate::grpc_impl::api::DomainManagedInstanceId,
    ) -> anyhow::Result<crate::function_instance::DomainManagedInstanceId> {
        Ok(uuid::Uuid::parse_str(&api_id.instance_id)?)
    }

    pub fn parse_start_component_response<ResourceIdType>(
        api_instance: &crate::grpc_impl::api::StartComponentResponse,
    ) -> anyhow::Result<crate::common::StartComponentResponse<ResourceIdType>>
    where
        crate::grpc_impl::api::InstanceIdVariant: ParseableId<ResourceIdType>,
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

    pub fn parse_patch_request(
        api_update: &crate::grpc_impl::api::PatchRequest,
    ) -> anyhow::Result<crate::common::PatchRequest> {
        Ok(crate::common::PatchRequest {
            function_id: uuid::Uuid::parse_str(&api_update.function_id)?,
            output_mapping: api_update
                .output_mapping
                .iter()
                .filter_map(
                    |(key, value)| match CommonConverters::parse_instance_id(value) {
                        Ok(val) => Some((key.clone(), val)),
                        Err(_) => None,
                    },
                )
                .collect(),
        })
    }

    pub fn serialize_response_error(
        crate_function: &crate::common::ResponseError,
    ) -> crate::grpc_impl::api::ResponseError {
        crate::grpc_impl::api::ResponseError {
            summary: crate_function.summary.clone(),
            detail: crate_function.detail.clone(),
        }
    }

    pub fn serialize_instance_id(
        instance_id: &crate::function_instance::InstanceId,
    ) -> crate::grpc_impl::api::InstanceId {
        crate::grpc_impl::api::InstanceId {
            node_id: instance_id.node_id.to_string(),
            function_id: instance_id.function_id.to_string(),
        }
    }

    pub fn serialize_event_timestamp(
        ts: &crate::function_instance::EventTimestamp,
    ) -> crate::grpc_impl::api::EventTimestamp {
        crate::grpc_impl::api::EventTimestamp {
            secs: ts.secs,
            nsecs: ts.nsecs,
        }
    }

    pub fn serialize_domain_managed_instance_id(
        instance_id: &crate::function_instance::DomainManagedInstanceId,
    ) -> crate::grpc_impl::api::DomainManagedInstanceId {
        crate::grpc_impl::api::DomainManagedInstanceId {
            instance_id: instance_id.to_string(),
        }
    }

    pub fn serialize_start_component_response<ComponentIdType: SerializeableId>(
        req: &crate::common::StartComponentResponse<ComponentIdType>,
    ) -> crate::grpc_impl::api::StartComponentResponse {
        match req {
            crate::common::StartComponentResponse::ResponseError(err) => {
                crate::grpc_impl::api::StartComponentResponse {
                    response_error: Some(CommonConverters::serialize_response_error(err)),
                    instance_id: None,
                }
            }
            crate::common::StartComponentResponse::InstanceId(id) => {
                crate::grpc_impl::api::StartComponentResponse {
                    response_error: None,
                    instance_id: Some(SerializeableId::serialize(id)),
                }
            }
        }
    }

    pub fn serialize_patch_request(
        crate_update: &crate::common::PatchRequest,
    ) -> crate::grpc_impl::api::PatchRequest {
        crate::grpc_impl::api::PatchRequest {
            function_id: crate_update.function_id.to_string(),
            output_mapping: crate_update
                .output_mapping
                .iter()
                .map(|(key, value)| (key.clone(), CommonConverters::serialize_instance_id(value)))
                .collect(),
        }
    }
}

impl From<&edgeless_api_core::event_metadata::EventMetadata>
    for crate::grpc_impl::api::EventSerializedMetadata
{
    fn from(value: &edgeless_api_core::event_metadata::EventMetadata) -> Self {
        let _words = value.trace_id().to_bytes();
        Self {
            trace_id: _words.to_vec(),
            span_id: value.span_id().to_bytes().to_vec(),
        }
    }
}

impl TryFrom<&crate::grpc_impl::api::EventSerializedMetadata>
    for edgeless_api_core::event_metadata::EventMetadata
{
    type Error = anyhow::Error;

    fn try_from(
        value: &crate::grpc_impl::api::EventSerializedMetadata,
    ) -> Result<Self, Self::Error> {
        let trace_id: [u8; 16] = value
            .clone()
            .trace_id
            .try_into()
            .map_err(|_| anyhow::anyhow!("Mismatched length"))?;
        let span_id: [u8; 8] = value
            .clone()
            .span_id
            .try_into()
            .map_err(|_| anyhow::anyhow!("Mismatched length"))?;
        Ok(Self::from_bytes(trace_id, span_id))
    }
}

#[cfg(test)]
mod tests {
    use edgeless_api_core::event_metadata::EventMetadata;
    use edgeless_api_core::instance_id::InstanceId;

    use super::*;
    use crate::common::PatchRequest;
    use crate::grpc_impl::api::EventSerializedMetadata;

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
            match CommonConverters::parse_patch_request(&CommonConverters::serialize_patch_request(
                &msg,
            )) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn from_conversion_test() {
        let inputs = vec![
            EventMetadata::from_uints(0, 0),
            EventMetadata::from_uints(0, 1),
            EventMetadata::from_uints(1, 0),
            EventMetadata::from_uints(1, 2),
            EventMetadata::from_uints(0x42a42bdecaf00005u128, 0x42a42bdecaf00006u64),
            EventMetadata::from_uints(std::u128::MAX, std::u64::MAX),
        ];
        for some_i in inputs {
            let ser = EventSerializedMetadata::from(&some_i);
            let some_o = EventMetadata::try_from(&ser);
            assert!(some_o.is_ok(), "cannot be an error");
            assert_eq!(some_i, some_o.unwrap(), "invalid identity transformation")
        }
    }
}
