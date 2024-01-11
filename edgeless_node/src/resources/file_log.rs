// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_dataplane::core::Message;
use std::io::prelude::*;

#[derive(Clone)]
pub struct FileLogResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<FileLogResourceProviderInner>>,
}

struct FileLogResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, FileLogResource>,
}

pub struct FileLogResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for FileLogResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl FileLogResource {
    async fn new(dataplane_handle: edgeless_dataplane::handle::DataplaneHandle, filename: &str) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;

        let mut outfile = std::fs::OpenOptions::new().create(true).write(true).append(true).open(filename)?;

        log::info!("FileLogResource created, writing to file: {}", filename);

        let handle = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                } = dataplane_handle.receive_next().await;
                let mut need_reply = false;
                let message_data = match message {
                    Message::Call(data) => {
                        need_reply = true;
                        data
                    }
                    Message::Cast(data) => data,
                    _ => {
                        continue;
                    }
                };

                log::debug!("{}", message_data);
                if let Err(e) = writeln!(outfile, "{}", message_data) {
                    log::error!("Could not write to file the message '{}': {}", message_data, e);
                }

                if need_reply {
                    dataplane_handle
                        .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply("".to_string()))
                        .await;
                }
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl FileLogResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(FileLogResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, FileLogResource>::new(),
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for FileLogResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        if let Some(filename) = instance_specification.configuration.get("filename") {
            let mut lck = self.inner.lock().await;

            let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id.clone()).await;

            match FileLogResource::new(dataplane_handle, filename).await {
                Ok(resource) => {
                    lck.instances.insert(new_id.clone(), resource);
                    return Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id));
                }
                Err(err) => {
                    return Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                        edgeless_api::common::ResponseError {
                            summary: "Invalid resource configuration".to_string(),
                            detail: Some(err.to_string()),
                        },
                    ));
                }
            }
        }
        Ok(edgeless_api::common::StartComponentResponse::ResponseError(
            edgeless_api::common::ResponseError {
                summary: "Invalid resource configuration".to_string(),
                detail: Some("Field 'filename' missing".to_string()),
            },
        ))
    }

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.inner.lock().await.instances.remove(&resource_id);
        Ok(())
    }

    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        // the resource has no channels: nothing to be patched
        Ok(())
    }
}
