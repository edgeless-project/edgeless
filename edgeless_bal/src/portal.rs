// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_dataplane::core::Message;

#[derive(PartialEq, Clone, Copy)]
enum Role {
    Sink,
    Source,
}

impl Role {
    pub fn new(val: &str) -> anyhow::Result<Self> {
        if val.to_ascii_lowercase() == "sink" {
            Ok(Self::Sink)
        } else if val.to_ascii_lowercase() == "source" {
            Ok(Self::Source)
        } else {
            anyhow::bail!("invalid role: {val}")
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self {
            Self::Sink => write!(fmt, "sink"),
            Self::Source => write!(fmt, "source"),
        }
    }
}

#[derive(PartialEq)]
enum Domain {
    Local,
    Portal,
}

impl Domain {
    pub fn new(val: &str) -> anyhow::Result<Self> {
        if val.to_ascii_lowercase() == "local" {
            Ok(Self::Local)
        } else if val.to_ascii_lowercase() == "portal" {
            Ok(Self::Portal)
        } else {
            anyhow::bail!("invalid domain: {val}")
        }
    }
}

impl std::fmt::Display for Domain {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self {
            Self::Local => write!(fmt, "local"),
            Self::Portal => write!(fmt, "portal"),
        }
    }
}

/// A PortalResource can be created only when the following happens
/// - a resource is created in the local domain
/// - a resource is created in the portal domain
/// - a patch command is issued
/// The `PortalPartialResource` holds partial information until then.
struct PortalPartialResource {
    /// Physical identifier used in the local domain.
    local_id: Option<edgeless_api::function_instance::InstanceId>,
    /// Physical identifier used in the portal domain.
    portal_id: Option<edgeless_api::function_instance::InstanceId>,
    /// Role.
    role: Role,
    /// Identifier of the target component.
    /// If `role` is `Sink`, then the target is another balancer
    /// in the domain-bal, otherwise it is a function/resource in a
    /// regular domain.
    target_id: Option<edgeless_api::function_instance::InstanceId>,
}

impl PortalPartialResource {
    pub fn new(role: Role) -> Self {
        Self {
            local_id: None,
            portal_id: None,
            role,
            target_id: None,
        }
    }

    pub fn matching(&self, component_id: &edgeless_api::function_instance::ComponentId) -> bool {
        if let Some(local_id) = &self.local_id {
            if local_id.function_id == *component_id {
                return true;
            }
        }
        if let Some(portal_id) = &self.portal_id {
            if portal_id.function_id == *component_id {
                return true;
            }
        }
        false
    }

    pub fn complete(
        &self,
    ) -> Option<(
        edgeless_api::function_instance::InstanceId,
        edgeless_api::function_instance::InstanceId,
        Role,
        edgeless_api::function_instance::InstanceId,
    )> {
        if self.local_id.is_some() && self.portal_id.is_some() && self.target_id.is_some() {
            Some((self.local_id.unwrap(), self.portal_id.unwrap(), self.role, self.target_id.unwrap()))
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct PortalResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<PortalResourceProviderInner>>,
}

pub struct PortalResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    local_dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    portal_dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    partial: std::collections::HashMap<u64, PortalPartialResource>,
    instances: std::collections::HashMap<u64, PortalResource>,
}

pub struct PortalResource {
    local_id: edgeless_api::function_instance::InstanceId,
    portal_id: edgeless_api::function_instance::InstanceId,
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for PortalResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

/// Portal resource, to teleport messages from one domain to another.
impl PortalResource {
    async fn new(
        local_dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        portal_dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        local_id: edgeless_api::function_instance::InstanceId,
        portal_id: edgeless_api::function_instance::InstanceId,
        role: Role,
        target_id: edgeless_api::function_instance::InstanceId,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> anyhow::Result<Self> {
        let (mut dataplane_in_handle, mut dataplane_out_handle) = if role == Role::Sink {
            (local_dataplane_handle, portal_dataplane_handle)
        } else {
            (portal_dataplane_handle, local_dataplane_handle)
        };
        let mut telemetry_handle = telemetry_handle;

        log::info!("Portal created with role {role}, local ID {local_id}, portal ID {portal_id}, target ID {target_id}",);

        let handle = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                    created,
                    metadata,
                } = dataplane_in_handle.receive_next().await;
                let started = edgeless_node::resources::observe_transfer(created, &mut telemetry_handle);

                let need_reply = match message {
                    Message::Call(msg) => {
                        let reply = dataplane_out_handle
                            .call(target_id, msg, &edgeless_api::function_instance::EventMetadata::empty_new_root())
                            .await;
                        dataplane_in_handle.reply(source_id, channel_id, reply, &metadata).await;
                        true
                    }
                    Message::Cast(msg) => {
                        dataplane_out_handle.send(target_id, msg, &metadata).await;
                        false
                    }
                    _ => {
                        continue;
                    }
                };

                edgeless_node::resources::observe_execution(started, &mut telemetry_handle, need_reply);
            }
        });

        Ok(Self {
            local_id,
            portal_id,
            join_handle: handle,
        })
    }
}

impl PortalResourceProvider {
    pub async fn new(
        local_dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        portal_dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(PortalResourceProviderInner {
                resource_provider_id,
                local_dataplane_provider,
                portal_dataplane_provider,
                telemetry_handle,
                partial: std::collections::HashMap::new(),
                instances: std::collections::HashMap::new(),
            })),
        }
    }

    async fn create_instance_if_complete(&mut self, id: u64) -> anyhow::Result<()> {
        let mut lck = self.inner.lock().await;

        if let Some(partial) = lck.partial.get(&id) {
            if let Some((local_id, portal_id, role, target_id)) = partial.complete() {
                lck.partial.remove(&id);
                let resource = PortalResource::new(
                    lck.local_dataplane_provider.get_handle_for(local_id).await,
                    lck.portal_dataplane_provider.get_handle_for(portal_id).await,
                    local_id,
                    portal_id,
                    role,
                    target_id,
                    lck.telemetry_handle.fork(std::collections::BTreeMap::from([(
                        "FUNCTION_ID".to_string(),
                        local_id.function_id.to_string(),
                    )])),
                )
                .await?;
                lck.instances.insert(id, resource);
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for PortalResourceProvider {
    /// Resource configuration:
    ///
    /// - role [string]: one of {sink, source};
    /// - domain [string]: one of {local, portal};
    /// - id [u64]: unique identifier of the resource to match local with portal
    ///
    /// All the fields are mandatory, no defaults.
    async fn start(
        &mut self,
        specs: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        // Read resource configuration.
        let role = Role::new(specs.configuration.get("role").unwrap_or(&String::from("unspecified")))?;
        let domain = Domain::new(specs.configuration.get("domain").unwrap_or(&String::from("unspecified")))?;
        let id = specs.configuration.get("id").unwrap_or(&String::default()).parse::<u64>()?;

        let ret = {
            let mut lck = self.inner.lock().await;
            let pid = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);

            let partial = lck.partial.entry(id).or_insert(PortalPartialResource::new(role));
            anyhow::ensure!(
                role == partial.role,
                "invalid configuration of portal resource ID {}: trying to change the role from {} to {}",
                id,
                partial.role,
                role
            );

            if domain == Domain::Local {
                anyhow::ensure!(
                    partial.local_id.is_none(),
                    "invalid configuration of portal resource ID {}: trying to change local ID from {} to {}",
                    id,
                    partial.local_id.unwrap(),
                    pid
                );
                partial.local_id = Some(pid);
            } else {
                anyhow::ensure!(
                    partial.portal_id.is_none(),
                    "invalid configuration of portal resource ID {}: trying to change porta, ID from {} to {}",
                    id,
                    partial.portal_id.unwrap(),
                    pid
                );
                partial.portal_id = Some(pid);
            }
            Ok(edgeless_api::common::StartComponentResponse::InstanceId(pid))
        };

        self.create_instance_if_complete(id).await?;

        ret
    }

    /// Remove partial and active instances matching `resource_id` as local
    /// or portal ID.
    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        let mut lck = self.inner.lock().await;
        lck.instances.retain(|_k, v| v.local_id != resource_id && v.portal_id != resource_id);
        lck.partial
            .retain(|_k, v| v.local_id != Some(resource_id) && v.portal_id != Some(resource_id));
        Ok(())
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        let id = if let Some(target_id) = update.output_mapping.get("out") {
            let mut lck = self.inner.lock().await;
            if let Some((id, partial_resource)) = lck.partial.iter_mut().find(|(_k, v)| v.matching(&update.function_id)) {
                partial_resource.target_id = Some(target_id.clone());
                *id
            } else {
                anyhow::bail!("could not patch portal resource {}: no matching resources", update.function_id);
            }
        } else {
            anyhow::bail!(
                "invalid patch command for portal resource {}: no 'out' channel specified",
                update.function_id
            );
        };

        self.create_instance_if_complete(id).await?;

        Ok(())
    }
}
