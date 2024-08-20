#[derive(Debug, PartialEq, Eq, allocative::Allocative, starlark::any::ProvidesStaticType, serde::Serialize, serde::Deserialize, Clone)]
pub enum Mapping {
    Unmapped,
    Direct(DirectTarget),
    Any(Vec<DirectTarget>),
    All(Vec<DirectTarget>),
    Topic(String),
}

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, allocative::Allocative, Clone)]
pub struct DirectTarget {
    pub target_component: String,
    pub port: String,
}

#[derive(Debug, PartialEq, Eq, allocative::Allocative, starlark::any::ProvidesStaticType, serde::Serialize, serde::Deserialize, Clone)]
pub struct PortGen<MappingType> {
    pub component_id: String,
    pub port_id: String,
    pub mapping: MappingType,
    pub klass: crate::port_class::PortSpec,
}

pub type Port = PortGen<std::rc::Rc<std::cell::RefCell<Mapping>>>;
pub type FrozenPort = PortGen<Mapping>;

#[starlark::values::starlark_value(type = "edgeless_frozen_port_C")]
impl<'v> starlark::values::StarlarkValue<'v> for FrozenPort {
    type Canonical = FrozenPort;
}

#[starlark::values::starlark_value(type = "edgeless_port_C")]
impl<'v> starlark::values::StarlarkValue<'v> for Port {
    type Canonical = Port;

    fn right_shift(
        &self,
        other: starlark::values::Value<'v>,
        heap: &'v starlark::values::Heap,
    ) -> Result<starlark::values::Value<'v>, starlark::Error> {
        *self.mapping.borrow_mut() = Mapping::Direct(DirectTarget {
            target_component: "Foo".to_string(),
            port: "bar".to_string(),
        });
        Ok(other)
    }
}

impl<'v> starlark::values::AllocValue<'v> for Port {
    fn alloc_value(self, heap: &'v starlark::values::Heap) -> starlark::values::Value<'v> {
        heap.alloc_complex(self)
    }
}

impl<V> std::fmt::Display for PortGen<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl starlark::values::Freeze for Port {
    type Frozen = FrozenPort;

    fn freeze(self, _freezer: &starlark::values::Freezer) -> anyhow::Result<Self::Frozen> {
        Ok(FrozenPort {
            component_id: self.component_id.clone(),
            port_id: self.port_id.clone(),
            mapping: self.mapping.borrow_mut().clone(),
            klass: self.klass.clone(),
        })
    }
}

unsafe impl<'v> starlark::values::Trace<'v> for Port {
    fn trace(&mut self, tracer: &starlark::values::Tracer<'v>) {
        todo!()
    }
}
