// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use serde::ser::SerializeTupleVariant;

#[derive(Clone)]
pub enum ActiveInstance {
    // 0: request
    // 1: [ (node_id, lid) ]
    Function(
        edgeless_api::function_instance::SpawnFunctionRequest,
        Vec<edgeless_api::function_instance::InstanceId>,
    ),

    // 0: request
    // 1: (node_id, lid)
    Resource(
        edgeless_api::resource_configuration::ResourceInstanceSpecification,
        edgeless_api::function_instance::InstanceId,
    ),
}

impl ActiveInstance {
    pub fn instance_ids(&self) -> Vec<edgeless_api::function_instance::InstanceId> {
        match self {
            Self::Function(_, ids) => ids.clone(),
            Self::Resource(_, id) => vec![*id],
        }
    }
    pub fn workflow_id(&self) -> String {
        match &self {
            Self::Function(spawn_function_request, _) => spawn_function_request.workflow_id.clone(),
            Self::Resource(resource_instance_specification, _) => resource_instance_specification.workflow_id.clone(),
        }
    }
}

impl serde::Serialize for ActiveInstance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            ActiveInstance::Function(ref req, ref ids) => {
                let mut tv = serializer.serialize_tuple_variant("ActiveInstance", 0, "Function", 2)?;
                tv.serialize_field(req)?;
                tv.serialize_field::<Vec<String>>(ids.iter().map(|x| x.to_string()).collect::<Vec<String>>().as_ref())?;
                tv.end()
            }
            ActiveInstance::Resource(ref req, ref id) => {
                let mut tv = serializer.serialize_tuple_variant("ActiveInstance", 1, "Resource", 2)?;
                tv.serialize_field(req)?;
                tv.serialize_field(id.to_string().as_str())?;
                tv.end()
            }
        }
    }
}

impl std::fmt::Display for ActiveInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ActiveInstance::Function(_req, instances) => write!(
                f,
                "function, instances {}",
                instances
                    .iter()
                    .map(|x| format!("node_id {}, lid {}", x.node_id, x.function_id))
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            ActiveInstance::Resource(req, instance_id) => write!(
                f,
                "resource class type {}, node_id {}, lid {}",
                req.class_type, instance_id.node_id, instance_id.function_id
            ),
        }
    }
}
