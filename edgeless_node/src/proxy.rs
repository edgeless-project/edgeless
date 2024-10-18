use futures::FutureExt;

#[derive(Clone)]
pub struct ProxyManager {
    sender: tokio::sync::mpsc::UnboundedSender<ProxyManagerRequest>,
    join_handle: std::sync::Arc<tokio::task::JoinHandle<()>>,
}

pub struct ProxyManagerTask {
    receiver: tokio::sync::mpsc::UnboundedReceiver<ProxyManagerRequest>,
    instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, ProxyInstance>,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
}

pub struct ProxyInstance {
    sender: tokio::sync::mpsc::UnboundedSender<ProxyInstanceRequest>,
    join_handle: tokio::task::JoinHandle<()>,
}

pub struct ProxyInstanceTask {
    control_receiver: tokio::sync::mpsc::UnboundedReceiver<ProxyInstanceRequest>,
    internal_dataplane: edgeless_dataplane::handle::DataplaneHandle,
    external_dataplane: edgeless_dataplane::handle::DataplaneHandle,
    inner_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Output>,
    inner_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Input>,
    external_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Output>,
    external_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::common::Input>,
}

pub enum ProxyInstanceRequest {
    Update(edgeless_api::proxy_instance::ProxySpec),
}

pub enum ProxyManagerRequest {
    Start(edgeless_api::proxy_instance::ProxySpec),
    Update(edgeless_api::proxy_instance::ProxySpec),
    Stop(edgeless_api::function_instance::InstanceId),
}

impl ProxyInstanceTask {
    async fn run(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                futures::select! {
                    internal_message = self.internal_dataplane.receive_next().fuse() => {
                        match internal_message.message {
                            edgeless_dataplane::core::Message::Cast(msg) => {
                                self.external_dataplane.send_alias(internal_message.target_port.0, msg).await;
                            },
                            edgeless_dataplane::core::Message::Call(msg) => {
                                let reply = self.external_dataplane.call_alias(internal_message.target_port.0, msg).await;
                                self.internal_dataplane.reply(internal_message.source_id, internal_message.channel_id, reply).await;
                            },
                            _ => {
                                // This should never happen
                                log::error!("Unhandled Message in Proxy.")
                            }
                        }
                        //
                    },
                    external_message = self.external_dataplane.receive_next().fuse() => {
                        match external_message.message {
                            edgeless_dataplane::core::Message::Cast(msg) => {
                                self.internal_dataplane.send_alias(external_message.target_port.0, msg).await;
                            },
                            edgeless_dataplane::core::Message::Call(msg) => {
                                let reply = self.internal_dataplane.call_alias(external_message.target_port.0, msg).await;
                                self.external_dataplane.reply(external_message.source_id, external_message.channel_id, reply).await;
                            },
                            _ => {
                                // This should never happen
                                log::error!("Unhandled Message in Proxy.")
                            }
                        }
                    },
                    control_request = self.control_receiver.recv().fuse() => {
                        if let Some(control_request) = control_request {
                            match control_request {
                                ProxyInstanceRequest::Update(proxy_spec) => {
                                    self.internal_dataplane.update_mapping(proxy_spec.inner_inputs.clone(), proxy_spec.inner_outputs.clone());
                                    self.external_dataplane.update_mapping(proxy_spec.external_inputs.clone(), proxy_spec.external_outputs.clone());
                                    self.inner_inputs = proxy_spec.inner_inputs;
                                    self.inner_outputs = proxy_spec.inner_outputs;
                                    self.external_inputs  = proxy_spec.external_inputs;
                                    self.external_outputs = proxy_spec.external_outputs;
                                }
                            }
                        } else {
                            return;
                        }
                    }
                }
            }
        })
    }
}

impl ProxyInstance {
    async fn create(
        spec: edgeless_api::proxy_instance::ProxySpec,
        internal_dataplane: edgeless_dataplane::handle::DataplaneHandle,
        external_dataplane: edgeless_dataplane::handle::DataplaneHandle,
    ) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<ProxyInstanceRequest>();

        let task = ProxyInstanceTask {
            control_receiver: receiver,
            internal_dataplane: internal_dataplane,
            external_dataplane: external_dataplane,
            inner_outputs: spec.inner_outputs,
            inner_inputs: spec.inner_inputs,
            external_outputs: spec.external_outputs,
            external_inputs: spec.external_inputs,
        };

        let join_handle = task.run().await;

        Self { sender, join_handle }
    }
}

impl ProxyManager {
    pub async fn start(dataplane_provider: edgeless_dataplane::handle::DataplaneProvider) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<ProxyManagerRequest>();

        let t = ProxyManagerTask {
            receiver,
            instances: std::collections::HashMap::new(),
            dataplane_provider,
        };

        let join_handle = t.run().await;

        Self {
            sender,
            join_handle: std::sync::Arc::new(join_handle),
        }
    }
}

impl ProxyManagerTask {
    async fn run(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                if let Some(req) = self.receiver.recv().await {
                    match req {
                        ProxyManagerRequest::Start(proxy_spec) => {
                            self.instances.insert(
                                proxy_spec.instance_id.clone(),
                                ProxyInstance::create(
                                    proxy_spec.clone(),
                                    self.dataplane_provider.get_handle_for(proxy_spec.instance_id).await,
                                    self.dataplane_provider
                                        .get_handle_for(edgeless_api::function_instance::InstanceId::new(proxy_spec.instance_id.node_id.clone()))
                                        .await,
                                )
                                .await,
                            );
                        }
                        ProxyManagerRequest::Update(proxy_spec) => {
                            if let Some(instance) = self.instances.get(&proxy_spec.instance_id) {
                                let res = instance.sender.send(ProxyInstanceRequest::Update(proxy_spec));
                                if res.is_err() {
                                    log::info!("Could not send message to proxy instance");
                                }
                            }
                        }
                        ProxyManagerRequest::Stop(instance_id) => {
                            self.instances.remove(&instance_id);
                        }
                    }
                } else {
                    log::info!("Proxy Stopped");
                    return;
                }
            }
        })
    }
}

#[async_trait::async_trait]
impl edgeless_api::proxy_instance::ProxyInstanceAPI for ProxyManager {
    async fn start(&mut self, request: edgeless_api::proxy_instance::ProxySpec) -> anyhow::Result<()> {
        self.sender.send(ProxyManagerRequest::Start(request)).unwrap();
        Ok(())
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.sender.send(ProxyManagerRequest::Stop(id)).unwrap();
        Ok(())
    }
    async fn patch(&mut self, update: edgeless_api::proxy_instance::ProxySpec) -> anyhow::Result<()> {
        self.sender.send(ProxyManagerRequest::Update(update)).unwrap();
        Ok(())
    }
}
