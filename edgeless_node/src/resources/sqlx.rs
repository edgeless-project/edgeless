// SPDX-FileCopyrightText: © 2024 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Chen Chen <cc2181@cam.ac.uk>
// SPDX-License-Identifier: MIT
use edgeless_dataplane::core::Message;
use serde::Deserialize;
use sqlx::{migrate::MigrateDatabase, FromRow, Sqlite, SqlitePool};
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
    async fn new(dataplane_handle: edgeless_dataplane::handle::DataplaneHandle, sqlx_url: &str, workflow_id: &String) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let sqlx_url = sqlx_url.to_string();

        let workflow_id = workflow_id.clone();
        let handle = tokio::spawn(async move {
            loop {
                let workflow_id = workflow_id.clone();

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

                if !Sqlite::database_exists(&sqlx_url).await.unwrap_or(false) {
                    println!("Creating database {}", sqlx_url);
                    match Sqlite::create_database(&sqlx_url).await {
                        Ok(_) => log::info!("Create sqlx db success"),
                        Err(error) => panic!("error: {}", error),
                    }
                } else {
                    // log::info!("sqlx Database exists");
                }

                let db = SqlitePool::connect(&sqlx_url).await.unwrap();

                let response = sqlx::query(
                    "CREATE TABLE IF NOT EXISTS WorkflowState (
                    id VARCHAR(255) PRIMARY KEY,
                    name VARCHAR(255)  NOT NULL,
                    result INTEGER NOT NULL,
                    timestamp VARCHAR(255),);",
                )
                .execute(&db)
                .await
                .unwrap();

                log::info!("create table in sql {:?}", response);

                if message_data.to_string().contains("SELECT") {
                    let result: sqlx::Result<WorkflowState, sqlx::Error> =
                        sqlx::query_as(message_data.as_str()).bind(workflow_id).fetch_one(&db).await;

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
                            log::info!("Response from database: {:?}", e.to_string());
                            dataplane_handle
                                .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply(e.to_string()))
                                .await;
                        }
                    }
                } else if message_data.to_string().contains("INSERT")
                    || message_data.to_string().contains("UPDATE")
                    || message_data.to_string().contains("DELETE")
                {
                    let result = sqlx::query(message_data.as_str()).bind(workflow_id).execute(&db).await;
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
                            dataplane_handle
                                .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply(e.to_string()))
                                .await;
                        }
                    }
                } else {
                    log::info!("Unknow operation in state management");
                    dataplane_handle
                        .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Err)
                        .await;
                };

                db.close().await;
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
        if let (Some(url), Some(_key), workflow_id) = (
            instance_specification.configuration.get("url"),
            instance_specification.configuration.get("key"),
            instance_specification.workflow_id,
        ) {
            let mut lck = self.inner.lock().await;
            let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;

            match SqlxResource::new(dataplane_handle, url, &workflow_id).await {
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

#[derive(Clone, FromRow, Debug, Deserialize)]
struct WorkflowState {
    id: String,
    name: String,
    result: i64,
    timestamp: String,
}

impl WorkflowState {
    fn to_string(&self) -> String {
        let data = format!(
            "id: {}, name: {}, result: {:?}, timestamp: {}",
            self.id, self.name, self.result, self.timestamp
        );
        data
    }
}
