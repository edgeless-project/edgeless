#[derive(Debug, PartialEq, Eq, allocative::Allocative, starlark::any::ProvidesStaticType, serde::Serialize, serde::Deserialize, Clone)]
pub struct EdgelessResourceClass {
    pub id: String,
    pub inputs: std::collections::HashMap<String, crate::port_class::PortSpec>,
    pub ouputs: std::collections::HashMap<String, crate::port_class::PortSpec>,
    pub inner_structure: super::inner_structure::InnerStructure,
}

starlark::starlark_simple_value!(EdgelessResourceClass);

impl std::fmt::Display for EdgelessResourceClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[starlark::values::starlark_value(type = "edgeless_resource_class_c")]
impl<'v> starlark::values::StarlarkValue<'v> for EdgelessResourceClass {}

impl<'v> starlark::values::UnpackValue<'v> for EdgelessResourceClass {
    fn unpack_value(value: starlark::values::Value<'v>) -> Option<Self> {
        starlark::values::ValueLike::downcast_ref::<EdgelessResourceClass>(value).cloned()
    }
}

#[starlark::starlark_module]
pub fn edgeless_resource_class(builder: &mut starlark::environment::GlobalsBuilder) {
    fn edgeless_resource_class(
        id: String,
        // version: String,
        outputs: starlark::values::list::UnpackList<crate::port_class::PortSpec>,
        inputs: starlark::values::list::UnpackList<crate::port_class::PortSpec>,
        inner_structure: starlark::values::list::UnpackList<super::inner_structure::Mapping>,
        heap: &'v starlark::values::Heap,
    ) -> anyhow::Result<starlark::values::Value> {
        Ok(heap.alloc(EdgelessResourceClass {
            id: id,
            // version: version,
            inputs: inputs.into_iter().map(|i| (i.id.clone(), i)).collect(),
            ouputs: outputs.into_iter().map(|o| (o.id.clone(), o)).collect(),
            inner_structure: inner_structure.into_iter().collect(),
        }))
    }
}
