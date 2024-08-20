use starlark::values::{list::UnpackList, ValueLike};

#[derive(Debug, PartialEq, Eq, allocative::Allocative, starlark::any::ProvidesStaticType, serde::Serialize, serde::Deserialize, Clone)]
pub struct EdgelessWorkflow {
    pub id: String,
    #[serde(alias = "functions")]
    pub actors: Vec<crate::actor::FrozenEdgelessActor>,
    pub resources: Vec<crate::resource::FrozenEdgelessResource>,
    pub annotations: std::collections::HashMap<String, String>,
}

starlark::starlark_simple_value!(EdgelessWorkflow);

impl std::fmt::Display for EdgelessWorkflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[starlark::values::starlark_value(type = "edgeless_workflow_c")]
impl<'v> starlark::values::StarlarkValue<'v> for EdgelessWorkflow {}

#[starlark::starlark_module]
pub fn edgeless_workflow(builder: &mut starlark::environment::GlobalsBuilder) {
    fn edgeless_workflow<'v>(
        id: String,
        items: UnpackList<starlark::values::Value<'v>>,
        annotations: starlark::values::dict::DictOf<String, String>,
        heap: &'v starlark::values::Heap,
    ) -> anyhow::Result<starlark::values::Value<'v>> {
        // for i in items {
        //     println!("{}", i.get_type())
        // }

        let mut actors = Vec::<crate::actor::FrozenEdgelessActor>::new();
        let mut resources = Vec::<crate::resource::FrozenEdgelessResource>::new();

        for value in items {
            if let Some(v) = value.downcast_ref::<crate::actor::EdgelessActor>() {
                let cloned = v.clone();
                actors.push(crate::actor::FrozenEdgelessActor {
                    id: cloned.id,
                    klass: cloned.klass,
                    outputs: cloned
                        .outputs
                        .into_iter()
                        .filter_map(|(o_id, o)| {
                            let mapping = o.mapping.borrow_mut().clone();
                            if let &super::port::Mapping::Unmapped = &mapping {
                                None
                            } else {
                                Some((
                                    o_id,
                                    crate::port::FrozenPort {
                                        component_id: o.component_id.clone(),
                                        port_id: o.port_id.clone(),
                                        mapping: mapping,
                                        klass: o.klass,
                                    },
                                ))
                            }
                        })
                        .collect(),
                    inputs: cloned
                        .inputs
                        .into_iter()
                        .filter_map(|(i_id, i)| {
                            let mapping = i.mapping.borrow_mut().clone();
                            if let &super::port::Mapping::Unmapped = &mapping {
                                None
                            } else {
                                Some((
                                    i_id,
                                    crate::port::FrozenPort {
                                        component_id: i.component_id.clone(),
                                        port_id: i.port_id.clone(),
                                        mapping: i.mapping.borrow_mut().clone(),
                                        klass: i.klass,
                                    },
                                ))
                            }
                        })
                        .collect(),
                    annotations: cloned.annotations,
                });
            } else if let Some(i) = value.downcast_ref::<crate::actor::FrozenEdgelessActor>() {
                actors.push(i.clone())
            } else if let Some(v) = value.downcast_ref::<crate::resource::EdgelessResource>() {
                let cloned = v.clone();
                resources.push(crate::resource::FrozenEdgelessResource {
                    id: cloned.id,
                    klass: cloned.klass,
                    outputs: cloned
                        .outputs
                        .into_iter()
                        .filter_map(|(o_id, o)| {
                            let mapping = o.mapping.borrow_mut().clone();
                            if let &super::port::Mapping::Unmapped = &mapping {
                                None
                            } else {
                                Some((
                                    o_id,
                                    crate::port::FrozenPort {
                                        component_id: o.component_id.clone(),
                                        port_id: o.port_id.clone(),
                                        mapping: mapping,
                                        klass: o.klass,
                                    },
                                ))
                            }
                        })
                        .collect(),
                    inputs: cloned
                        .inputs
                        .into_iter()
                        .filter_map(|(i_id, i)| {
                            let mapping = i.mapping.borrow_mut().clone();
                            if let &super::port::Mapping::Unmapped = &mapping {
                                None
                            } else {
                                Some((
                                    i_id,
                                    crate::port::FrozenPort {
                                        component_id: i.component_id.clone(),
                                        port_id: i.port_id.clone(),
                                        mapping: mapping,
                                        klass: i.klass,
                                    },
                                ))
                            }
                        })
                        .collect(),
                    configurations: cloned.configurations,
                });
            } else if let Some(i) = value.downcast_ref::<crate::resource::FrozenEdgelessResource>() {
                resources.push(i.clone())
            }
        }

        Ok(heap.alloc(EdgelessWorkflow {
            id: id,
            actors: actors,
            resources: resources,
            annotations: annotations.collect_entries().into_iter().collect(),
        }))
    }
}
