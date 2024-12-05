// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use futures::{Future, SinkExt};

#[cfg(test)]
pub mod test;

pub struct Orchestrator {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

#[derive(Debug, Clone)]
pub struct NewNodeData {
    pub node_id: uuid::Uuid,
    pub agent_url: String,
    pub invocation_url: String,
    pub resource_providers: Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
    pub capabilities: edgeless_api::node_registration::NodeCapabilities,
}

impl NewNodeData {
    pub fn to_string_short(&self) -> String {
        format!(
            "node_id {}, agent URL {}, invocation URL {}",
            self.node_id, self.agent_url, self.invocation_url
        )
    }
}

pub enum OrchestratorRequest {
    StartFunction(
        edgeless_api::function_instance::SpawnFunctionRequest,
        tokio::sync::oneshot::Sender<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>>,
        >,
    ),
    StopFunction(edgeless_api::function_instance::DomainManagedInstanceId),
    StartResource(
        edgeless_api::resource_configuration::ResourceInstanceSpecification,
        tokio::sync::oneshot::Sender<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>>,
        >,
    ),
    StopResource(edgeless_api::function_instance::DomainManagedInstanceId),
    Patch(edgeless_api::common::PatchRequest),
    AddNode(NewNodeData),
    DelNode(uuid::Uuid),
}

pub(crate) enum InternalRequest {
    Poll(),
}

pub struct OrchestratorClient {
    function_instance_client: Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::DomainManagedInstanceId>>,
    resource_configuration_client:
        Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::DomainManagedInstanceId>>,
}

impl edgeless_api::outer::orc::OrchestratorAPI for OrchestratorClient {
    fn function_instance_api(
        &mut self,
    ) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::DomainManagedInstanceId>> {
        self.function_instance_client.clone()
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::DomainManagedInstanceId>> {
        self.resource_configuration_client.clone()
    }
}

#[derive(Clone)]
pub struct OrchestratorFunctionInstanceOrcClient {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

#[derive(Clone)]
pub struct ResourceConfigurationClient {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

impl Orchestrator {
    pub async fn new(
        settings: crate::EdgelessOrcBaselineSettings,
        proxy: std::sync::Arc<tokio::sync::Mutex<dyn super::proxy::Proxy>>,
        subscriber_sender: futures::channel::mpsc::UnboundedSender<super::domain_subscriber::DomainSubscriberRequest>,
    ) -> (
        Self,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
    ) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let (internal_sender, internal_receiver) = futures::channel::mpsc::unbounded();
        let main_task = Box::pin(async move {
            let mut orchestrator_task = super::orchestrator_task::OrchestratorTask::new(
                receiver,
                internal_receiver,
                settings,
                std::collections::HashMap::new(),
                std::collections::HashMap::new(),
                proxy,
                subscriber_sender,
            )
            .await;
            orchestrator_task.run().await;
        });

        let refresh_task = Box::pin(async move {
            let mut sender = internal_sender;
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                let _ = sender.send(InternalRequest::Poll()).await;
            }
        });

        (Orchestrator { sender }, main_task, refresh_task)
    }

    #[cfg(test)]
    pub async fn new_with_clients(
        settings: crate::EdgelessOrcBaselineSettings,
        clients: std::collections::HashMap<uuid::Uuid, crate::client_desc::ClientDesc>,
        resource_providers: std::collections::HashMap<String, crate::resource_provider::ResourceProvider>,
        subscriber_sender: futures::channel::mpsc::UnboundedSender<super::domain_subscriber::DomainSubscriberRequest>,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let (_internal_sender, internal_receiver) = futures::channel::mpsc::unbounded();
        let main_task = Box::pin(async move {
            let mut orchestrator_task = crate::orchestrator_task::OrchestratorTask::new(
                receiver,
                internal_receiver,
                settings,
                clients,
                resource_providers,
                std::sync::Arc::new(tokio::sync::Mutex::new(super::proxy_none::ProxyNone {})),
                subscriber_sender,
            )
            .await;
            orchestrator_task.run().await;
        });

        (Orchestrator { sender }, main_task)
    }

    pub fn get_sender(&mut self) -> futures::channel::mpsc::UnboundedSender<OrchestratorRequest> {
        self.sender.clone()
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::outer::orc::OrchestratorAPI + Send> {
        Box::new(OrchestratorClient {
            function_instance_client: Box::new(OrchestratorFunctionInstanceOrcClient { sender: self.sender.clone() }),
            resource_configuration_client: Box::new(ResourceConfigurationClient { sender: self.sender.clone() }),
        })
    }
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::DomainManagedInstanceId>
    for OrchestratorFunctionInstanceOrcClient
{
    async fn start(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>> {
        log::debug!("FunctionInstance::start() {:?}", request);
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>>,
        >();
        if let Err(err) = self.sender.send(OrchestratorRequest::StartFunction(request, reply_sender)).await {
            return Err(anyhow::anyhow!(
                "Orchestrator channel error when creating a function instance: {}",
                err.to_string()
            ));
        }
        match reply_receiver.await {
            Ok(f_id) => f_id,
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when creating a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn stop(&mut self, id: edgeless_api::function_instance::DomainManagedInstanceId) -> anyhow::Result<()> {
        log::debug!("FunctionInstance::stop() {:?}", id);
        match self.sender.send(OrchestratorRequest::StopFunction(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when stopping a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        log::debug!("FunctionInstance::patch() {:?}", update);
        match self.sender.send(OrchestratorRequest::Patch(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::DomainManagedInstanceId>
    for ResourceConfigurationClient
{
    async fn start(
        &mut self,
        request: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>> {
        log::debug!("ResourceConfigurationAPI::start() {:?}", request);
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>>,
        >();
        if let Err(err) = self.sender.send(OrchestratorRequest::StartResource(request, reply_sender)).await {
            return Err(anyhow::anyhow!(
                "Orchestrator channel error when starting a resource: {}",
                err.to_string()
            ));
        }
        match reply_receiver.await {
            Ok(f_id) => f_id,
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when starting a resource: {}",
                err.to_string()
            )),
        }
    }

    async fn stop(&mut self, id: edgeless_api::function_instance::DomainManagedInstanceId) -> anyhow::Result<()> {
        log::debug!("ResourceConfigurationAPI::stop() {:?}", id);
        match self.sender.send(OrchestratorRequest::StopResource(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when stopping a resource: {}",
                err.to_string()
            )),
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        log::debug!("ResourceConfigurationAPI::patch() {:?}", update);
        match self.sender.send(OrchestratorRequest::Patch(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}
