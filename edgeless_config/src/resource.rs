#[derive(Debug, PartialEq, Eq, allocative::Allocative, starlark::any::ProvidesStaticType, serde::Serialize, serde::Deserialize, Clone)]
pub struct EdgelessResourceGen<PortType> {
    pub id: String,
    pub klass: crate::resource_class::EdgelessResourceClass,
    pub outputs: std::collections::HashMap<String, PortType>,
    pub inputs: std::collections::HashMap<String, PortType>,
    pub configurations: std::collections::HashMap<String, String>,
}

pub type EdgelessResource = EdgelessResourceGen<crate::port::Port>;
pub type FrozenEdgelessResource = EdgelessResourceGen<crate::port::FrozenPort>;

impl<PortType> std::fmt::Display for EdgelessResourceGen<PortType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<'v> starlark::values::UnpackValue<'v> for FrozenEdgelessResource {
    fn unpack_value(value: starlark::values::Value<'v>) -> Option<Self> {
        starlark::values::ValueLike::downcast_ref::<FrozenEdgelessResource>(value).cloned()
    }
}

#[starlark::values::starlark_value(type = "edgeless_frozen_resource_c")]
impl<'v> starlark::values::StarlarkValue<'v> for EdgelessResource {
    type Canonical = EdgelessResource;

    fn get_attr(&self, attr_id: &str, heap: &'v starlark::values::Heap) -> std::option::Option<starlark::values::Value<'v>> {
        self.outputs
            .get(attr_id)
            .or(self.inputs.get(attr_id))
            .and_then(|x| Some(heap.alloc(x.clone())))
    }

    fn has_attr(&self, attribute: &str, heap: &'v starlark::values::Heap) -> bool {
        self.outputs.contains_key(attribute) || self.inputs.contains_key(attribute)
    }

    fn dir_attr(&self) -> Vec<String> {
        let mut inputs: Vec<_> = self.inputs.iter().map(|(i_id, _)| i_id.clone()).collect();
        let mut outputs: Vec<_> = self.outputs.iter().map(|(o_id, _)| o_id.clone()).collect();
        inputs.append(&mut outputs);
        inputs
    }
}

#[starlark::values::starlark_value(type = "edgeless_resource_c")]
impl<'v> starlark::values::StarlarkValue<'v> for FrozenEdgelessResource {
    type Canonical = FrozenEdgelessResource;
}

impl<'v> starlark::values::Freeze for EdgelessResource {
    type Frozen = FrozenEdgelessResource;
    fn freeze(self, freezer: &starlark::values::Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(EdgelessResourceGen::<crate::port::FrozenPort> {
            id: self.id,
            klass: self.klass,
            outputs: self.outputs.into_iter().map(|(o_id, o)| (o_id, o.freeze(freezer).unwrap())).collect(),
            inputs: self.inputs.into_iter().map(|(i_id, i)| (i_id, i.freeze(freezer).unwrap())).collect(),
            configurations: self.configurations,
        })
    }
}

unsafe impl<'v> starlark::values::Trace<'v> for EdgelessResource {
    fn trace(&mut self, tracer: &starlark::values::Tracer<'v>) {
        todo!()
    }
}

impl<'v> starlark::values::AllocValue<'v> for EdgelessResource {
    fn alloc_value(self, heap: &'v starlark::values::Heap) -> starlark::values::Value<'v> {
        heap.alloc_complex(self)
    }
}

impl<'v> starlark::values::AllocValue<'v> for FrozenEdgelessResource {
    fn alloc_value(self, heap: &'v starlark::values::Heap) -> starlark::values::Value<'v> {
        heap.alloc_simple(self)
    }
}

#[starlark::starlark_module]
pub fn edgeless_resource(builder: &mut starlark::environment::GlobalsBuilder) {
    fn edgeless_resource<'v>(
        id: String,
        klass: crate::resource_class::EdgelessResourceClass,
        configurations: starlark::values::dict::DictOf<String, String>,
        heap: &'v starlark::values::Heap,
    ) -> anyhow::Result<starlark::values::Value<'v>> {
        Ok(heap.alloc(EdgelessResource {
            id: id.clone(),
            klass: klass.clone(),
            outputs: klass
                .ouputs
                .iter()
                .map(|(port_id, port_spec)| {
                    (
                        port_id.clone(),
                        crate::port::Port {
                            component_id: id.clone(),
                            port_id: id.clone(),
                            klass: port_spec.clone(),
                            mapping: std::rc::Rc::new(std::cell::RefCell::new(crate::port::Mapping::Unmapped)),
                        },
                    )
                })
                .collect(),
            inputs: klass
                .inputs
                .iter()
                .map(|(port_id, port_spec)| {
                    (
                        port_id.clone(),
                        crate::port::Port {
                            component_id: id.clone(),
                            port_id: id.clone(),
                            klass: port_spec.clone(),
                            mapping: std::rc::Rc::new(std::cell::RefCell::new(crate::port::Mapping::Unmapped)),
                        },
                    )
                })
                .collect(),
            configurations: configurations.collect_entries().into_iter().collect(),
        }))
    }
}
