// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use serde::ser::SerializeTupleVariant;

#[derive(Clone)]
pub enum ActiveInstance {
    // 0: request
    // 1: [ (node_id, int_fid) ]
    Function(
        edgeless_api::function_instance::SpawnFunctionRequest,
        Vec<edgeless_api::function_instance::InstanceId>,
    ),

    // 0: request
    // 1: (node_id, int_fid)
    Resource(
        edgeless_api::resource_configuration::ResourceInstanceSpecification,
        edgeless_api::function_instance::InstanceId,
    ),
}

impl ActiveInstance {
    pub fn instance_ids(&self) -> Vec<edgeless_api::function_instance::InstanceId> {
        match self {
            ActiveInstance::Function(_, ids) => ids.clone(),
            ActiveInstance::Resource(_, id) => vec![*id],
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
                    .map(|x| format!("node_id {}, int_fid {}", x.node_id, x.function_id))
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            ActiveInstance::Resource(req, instance_id) => write!(
                f,
                "resource class type {}, node_id {}, function_id {}",
                req.class_type, instance_id.node_id, instance_id.function_id
            ),
        }
    }
}
