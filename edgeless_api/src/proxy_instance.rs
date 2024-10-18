#[derive(Clone)]
pub struct ProxySpec {
    pub instance_id: crate::function_instance::InstanceId,
    pub inner_outputs: std::collections::HashMap<super::function_instance::PortId, crate::common::Output>,
    pub inner_inputs: std::collections::HashMap<super::function_instance::PortId, crate::common::Input>,
    pub external_outputs: std::collections::HashMap<super::function_instance::PortId, crate::common::Output>,
    pub external_inputs: std::collections::HashMap<super::function_instance::PortId, crate::common::Input>,
}

#[async_trait::async_trait]
pub trait ProxyInstanceAPI: ProxyInstanceAPIClone + Send + Sync {
    async fn start(&mut self, request: ProxySpec) -> anyhow::Result<()>;
    async fn stop(&mut self, id: crate::function_instance::InstanceId) -> anyhow::Result<()>;
    async fn patch(&mut self, update: ProxySpec) -> anyhow::Result<()>;
}

// https://stackoverflow.com/a/30353928
pub trait ProxyInstanceAPIClone {
    fn clone_box(&self) -> Box<dyn ProxyInstanceAPI>;
}
impl<T> ProxyInstanceAPIClone for T
where
    T: 'static + ProxyInstanceAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn ProxyInstanceAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn ProxyInstanceAPI> {
    fn clone(&self) -> Box<dyn ProxyInstanceAPI> {
        self.clone_box()
    }
}
