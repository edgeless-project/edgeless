// SPDX-FileCopyrightText: © 2024 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Chen Chen <cc2181@cam.ac.uk>
// SPDX-License-Identifier: MIT
use edgeless_dataplane::core::Message;
use sqlx::{FromRow, Row, SqlitePool};
use tokio;

#[derive(Clone)]
pub struct SqlxResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<SqlxResourceProviderInner>>,
}

pub struct SqlxResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, SqlxResource>,
}

pub struct SqlxResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for SqlxResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl SqlxResource {
    async fn new(dataplane_handle: edgeless_dataplane::handle::DataplaneHandle, sqlx_url: &str) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let sqlx_url = sqlx_url.to_string();

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

                let db = SqlitePool::connect(&sqlx_url).await.unwrap();

                if message_data.to_string().contains("SELECT") {
                    let result: sqlx::Result<Workflow, sqlx::Error> = sqlx::query_as(message_data.as_str()).fetch_one(&db).await;

                    match result {
                        Ok(response) => {
                            log::info!("Response from database: {:?}", response.to_string());
                            if need_reply {
                                dataplane_handle
                                    .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply(response.to_string()))
                                    .await;
                            }
                        }
                        Err(e) => {
                            log::info!("Response from database: {:?}", e.to_string())
                        }
                    }
                } else if message_data.to_string().contains("INSERT")
                    || message_data.to_string().contains("UPDATE")
                    || message_data.to_string().contains("DELETE")
                {
                    let result = sqlx::query(message_data.as_str()).execute(&db).await;
                    match result {
                        Ok(response) => {
                            log::info!("Response from database: {:?}", response);
                            if need_reply {
                                let res = format!(
                                    "rows_affected: {}, last_insert_rowid: {}.",
                                    response.rows_affected(),
                                    response.last_insert_rowid()
                                );
                                dataplane_handle
                                    .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply(res))
                                    .await;
                            }
                        }

                        Err(e) => {
                            log::info!("Error from state management: {:?}", e);
                        }
                    }
                } else {
                    log::info!("Unknow operation in state management");
                };
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl SqlxResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(SqlxResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, SqlxResource>::new(),
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for SqlxResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        if let (Some(url), Some(_key)) = (
            instance_specification.configuration.get("url"),
            instance_specification.configuration.get("key"),
        ) {
            let mut lck = self.inner.lock().await;
            let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;

            match SqlxResource::new(dataplane_handle, url).await {
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
                detail: Some("One of the fields 'url' is missing".to_string()),
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

#[derive(Clone, FromRow, Debug)]
struct Workflow {
    id: i64,
    name: String,
    result: i64,
}

impl Workflow {
    fn to_string(&self) -> String {
        let data = format!("id: {}, name: {}, result: {:?},", self.id, self.name, self.result);
        data
    }
}
