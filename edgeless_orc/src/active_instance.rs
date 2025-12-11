// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use serde::ser::SerializeTupleVariant;

/// ActiveInstance of a function or resource.
#[derive(Clone, Debug)]
pub enum ActiveInstance {
    // 0: request
    // 1: [ ((node_id, lid), is_used) ] - is_used indicates whether the instance is currently used by a workflow or kept as a hot-standby. Hot-standbys are running on other nodes.
    Function(
        edgeless_api::function_instance::SpawnFunctionRequest,
        Vec<(edgeless_api::function_instance::InstanceId, bool)>,
    ),

    // 0: request
    // 1: (node_id, lid)
    Resource(
        edgeless_api::resource_configuration::ResourceInstanceSpecification,
        edgeless_api::function_instance::InstanceId,
    ),
}

impl ActiveInstance {
    /// Return the physical identifier(s) associated with this instance.
    pub fn instance_ids(&self) -> Vec<(edgeless_api::function_instance::InstanceId, bool)> {
        match self {
            Self::Function(_, ids) => ids.clone(),
            Self::Resource(_, id) => vec![(*id, true)], // resources are always warm as we don't do redundancy for them
        }
    }
    pub fn instance_ids_mut(&mut self) -> &mut Vec<(edgeless_api::function_instance::InstanceId, bool)> {
        match self {
            Self::Function(_, ids) => ids,
            Self::Resource(_, _) => {
                panic!("Cannot get mutable reference to instance ids of a resource instance - there is always only one instance");
            }
        }
    }
    /// Return the workflow identifier of this instance.
    pub fn workflow_id(&self) -> String {
        match &self {
            Self::Function(spawn_function_request, _) => spawn_function_request.workflow_id.clone(),
            Self::Resource(resource_instance_specification, _) => resource_instance_specification.workflow_id.clone(),
        }
    }
    /// Return a stripped copy of the instance.
    /// The return value is different from the original one only if it is
    /// a function instance of type RUST_WASM, in which case the bytecode of the
    /// function is removed from the return value.
    pub fn strip(&self) -> Self {
        match self {
            Self::Function(spec, ids) => Self::Function(spec.strip(), ids.clone()),
            Self::Resource(_, _) => self.clone(),
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
                tv.serialize_field::<Vec<String>>(ids.iter().map(|x| format!("({}, {})", x.0, x.1)).collect::<Vec<String>>().as_ref())?;
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
                    .map(|x| format!("node_id {}, lid {}, is_warm {}", x.0.node_id, x.0.function_id, x.1))
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
