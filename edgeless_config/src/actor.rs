#[derive(Debug, PartialEq, Eq, allocative::Allocative, starlark::any::ProvidesStaticType, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgelessActorGen<PortType> {
    pub id: String,
    #[serde(rename = "function_type")]
    pub klass: crate::actor_class::EdgelessActorClass,
    pub outputs: std::collections::HashMap<String, PortType>,
    pub inputs: std::collections::HashMap<String, PortType>,
    pub annotations: std::collections::HashMap<String, String>,
}

pub type EdgelessActor = EdgelessActorGen<crate::port::Port>;
pub type FrozenEdgelessActor = EdgelessActorGen<crate::port::FrozenPort>;

impl<PortType> std::fmt::Display for EdgelessActorGen<PortType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<'v> starlark::values::UnpackValue<'v> for FrozenEdgelessActor {
    fn unpack_value(value: starlark::values::Value<'v>) -> Option<Self> {
        starlark::values::ValueLike::downcast_ref::<FrozenEdgelessActor>(value).cloned()
    }
}

#[starlark::values::starlark_value(type = "edgeless_frozen_actor_c")]
impl<'v> starlark::values::StarlarkValue<'v> for EdgelessActor {
    type Canonical = EdgelessActor;

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

#[starlark::values::starlark_value(type = "edgeless_actor_c")]
impl<'v> starlark::values::StarlarkValue<'v> for FrozenEdgelessActor {
    type Canonical = FrozenEdgelessActor;
}

impl<'v> starlark::values::Freeze for EdgelessActor {
    type Frozen = FrozenEdgelessActor;
    fn freeze(self, freezer: &starlark::values::Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(EdgelessActorGen::<crate::port::FrozenPort> {
            id: self.id,
            klass: self.klass,
            outputs: self.outputs.into_iter().map(|(o_id, o)| (o_id, o.freeze(freezer).unwrap())).collect(),
            inputs: self.inputs.into_iter().map(|(i_id, i)| (i_id, i.freeze(freezer).unwrap())).collect(),
            annotations: self.annotations,
        })
    }
}

unsafe impl<'v> starlark::values::Trace<'v> for EdgelessActor {
    fn trace(&mut self, tracer: &starlark::values::Tracer<'v>) {
        todo!()
    }
}

impl<'v> starlark::values::AllocValue<'v> for EdgelessActor {
    fn alloc_value(self, heap: &'v starlark::values::Heap) -> starlark::values::Value<'v> {
        heap.alloc_complex(self)
    }
}

impl<'v> starlark::values::AllocValue<'v> for FrozenEdgelessActor {
    fn alloc_value(self, heap: &'v starlark::values::Heap) -> starlark::values::Value<'v> {
        heap.alloc_simple(self)
    }
}

#[starlark::starlark_module]
pub fn edgeless_actor(builder: &mut starlark::environment::GlobalsBuilder) {
    fn edgeless_actor<'v>(
        id: String,
        klass: crate::actor_class::EdgelessActorClass,
        annotations: starlark::values::dict::DictOf<String, String>,
        heap: &'v starlark::values::Heap,
    ) -> anyhow::Result<starlark::values::Value<'v>> {
        Ok(heap.alloc(EdgelessActor {
            id: id.clone(),
            klass: klass.clone(),
            outputs: klass
                .outputs
                .iter()
                .map(|(port_id, port_spec)| {
                    (
                        port_id.clone(),
                        crate::port::Port {
                            component_id: id.clone(),
                            port_id: port_id.clone(),
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
                            port_id: port_id.clone(),
                            klass: port_spec.clone(),
                            mapping: std::rc::Rc::new(std::cell::RefCell::new(crate::port::Mapping::Unmapped)),
                        },
                    )
                })
                .collect(),
            annotations: annotations.collect_entries().into_iter().collect(),
        }))
    }
}
