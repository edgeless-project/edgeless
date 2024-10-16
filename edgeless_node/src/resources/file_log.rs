// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_dataplane::core::Message;
use std::io::prelude::*;

pub struct FileLogResourceSpec {}

impl super::resource_provider_specs::ResourceProviderSpecs for FileLogResourceSpec {
    fn class_type(&self) -> String {
        String::from("file-log")
    }

    fn outputs(&self) -> Vec<String> {
        vec![]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([
            (
                String::from("add-source-id"),
                String::from("If specified adds the InstanceId of the source component"),
            ),
            (String::from("add-timestamp"), String::from("If specified adds a timestamp")),
        ])
    }

    fn version(&self) -> String {
        String::from("1.0")
    }
}

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
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        filename: &str,
        add_source_id: bool,
        add_timestamp: bool,
    ) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;

        let mut outfile = std::fs::OpenOptions::new().create(true).append(true).open(filename)?;

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

                // Compose the line piece by piece.
                let mut line = "".to_string();
                if add_timestamp {
                    line.push_str(format!("{} ", chrono::Utc::now().to_rfc3339()).as_str());
                }
                if add_source_id {
                    line.push_str(format!("{} ", source_id).as_str());
                }
                line.push_str(&message_data);

                // Dump the line to the output file.
                log::debug!("{}", line);
                if let Err(e) = writeln!(outfile, "{}", line) {
                    log::error!("Could not write to file the message '{}': {}", line, e);
                }

                // Reply to the caller if the resource instance was called.
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
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;

            match FileLogResource::new(
                dataplane_handle,
                filename,
                instance_specification.configuration.contains_key("add-source-id"),
                instance_specification.configuration.contains_key("add-timestamp"),
            )
            .await
            {
                Ok(resource) => {
                    lck.instances.insert(new_id, resource);
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
