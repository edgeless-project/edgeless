pub type InnerStructure = Vec<Mapping>;

#[derive(Debug, PartialEq, Eq, allocative::Allocative, Clone, starlark::any::ProvidesStaticType, serde::Serialize, serde::Deserialize)]
pub struct Mapping {
    pub source: MappingNode,
    pub dests: Vec<MappingNode>,
}

#[derive(Debug, PartialEq, Eq, allocative::Allocative, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "port_id")]
pub enum MappingNode {
    #[serde(rename = "SIDE_EFFECT")]
    SideEffect,
    #[serde(rename = "PORT")]
    Port(String),
}

starlark::starlark_simple_value!(Mapping);

impl std::fmt::Display for Mapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[starlark::values::starlark_value(type = "edgeless_mapping_c")]
impl<'v> starlark::values::StarlarkValue<'v> for Mapping {}

impl<'v> starlark::values::UnpackValue<'v> for Mapping {
    fn unpack_value(value: starlark::values::Value<'v>) -> Option<Self> {
        starlark::values::ValueLike::downcast_ref::<Mapping>(value).cloned()
    }
}

#[starlark::starlark_module]
pub fn edgeless_inner_structure(builder: &mut starlark::environment::GlobalsBuilder) {
    fn source(output_port_id: String, heap: &'v starlark::values::Heap) -> anyhow::Result<starlark::values::Value> {
        Ok(heap.alloc(super::inner_structure::Mapping {
            source: MappingNode::SideEffect,
            dests: vec![MappingNode::Port(output_port_id)],
        }))
    }

    fn sink(input_port_id: String, heap: &'v starlark::values::Heap) -> anyhow::Result<starlark::values::Value> {
        Ok(heap.alloc(Mapping {
            source: MappingNode::Port(input_port_id),
            dests: vec![MappingNode::SideEffect],
        }))
    }

    fn link(
        input_port_id: String,
        output_port_ids: starlark::values::list::UnpackList<String>,
        heap: &'v starlark::values::Heap,
    ) -> anyhow::Result<starlark::values::Value> {
        Ok(heap.alloc(Mapping {
            source: MappingNode::Port(input_port_id),
            dests: output_port_ids.into_iter().map(|port| MappingNode::Port(port)).collect(),
        }))
    }
}
