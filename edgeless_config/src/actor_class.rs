use starlark::values::list::UnpackList;

#[derive(Debug, PartialEq, Eq, allocative::Allocative, starlark::any::ProvidesStaticType, serde::Serialize, serde::Deserialize, Clone)]
pub struct EdgelessActorClass {
    pub id: String,
    pub version: String,
    pub inputs: std::collections::HashMap<String, crate::port_class::PortSpec>,
    pub outputs: std::collections::HashMap<String, crate::port_class::PortSpec>,
    pub inner_structure: super::inner_structure::InnerStructure,
    #[serde(flatten)]
    pub code: Option<crate::files::File>,
    pub code_type: String,
}

starlark::starlark_simple_value!(EdgelessActorClass);

impl std::fmt::Display for EdgelessActorClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[starlark::values::starlark_value(type = "edgeless_actor_class_c")]
impl<'v> starlark::values::StarlarkValue<'v> for EdgelessActorClass {}

impl<'v> starlark::values::UnpackValue<'v> for EdgelessActorClass {
    fn unpack_value(value: starlark::values::Value<'v>) -> Option<Self> {
        starlark::values::ValueLike::downcast_ref::<crate::actor_class::EdgelessActorClass>(value).cloned()
    }
}

#[starlark::starlark_module]
pub fn edgeless_actor_class(builder: &mut starlark::environment::GlobalsBuilder) {
    fn edgeless_actor_class<'v>(
        id: String,
        version: String,
        outputs: UnpackList<crate::port_class::PortSpec>,
        inputs: UnpackList<crate::port_class::PortSpec>,
        inner_structure: UnpackList<super::inner_structure::Mapping>,
        code: Option<crate::files::File>,
        code_type: String,
        heap: &'v starlark::values::Heap,
    ) -> anyhow::Result<starlark::values::Value<'v>> {
        Ok(heap.alloc(EdgelessActorClass {
            id: id,
            version: version,
            inputs: inputs.into_iter().map(|i| (i.id.clone(), i)).collect(),
            outputs: outputs.into_iter().map(|o| (o.id.clone(), o)).collect(),
            inner_structure: inner_structure.into_iter().collect(),
            code: code,
            code_type: code_type,
        }))
    }
}
