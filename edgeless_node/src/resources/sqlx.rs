// SPDX-FileCopyrightText: © 2024 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Chen Chen <cc2181@cam.ac.uk>
// SPDX-License-Identifier: MIT
use edgeless_dataplane::core::Message;
use sqlx::{migrate::MigrateDatabase, FromRow, Sqlite, SqlitePool};
use tokio;

pub struct SqlxResourceSpec {}

impl super::resource_provider_specs::ResourceProviderSpecs for SqlxResourceSpec {
    fn class_type(&self) -> String {
        String::from("sqlx")
    }

    fn description(&self) -> String {
        r"Perform operations on an SQLite database".to_string()
    }

    fn outputs(&self) -> Vec<String> {
        vec![]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([(
            String::from("url"),
            String::from("URL of the SQLite database"),
        )])
    }

    fn version(&self) -> String {
        String::from("1.1")
    }
}

#[derive(Clone)]
pub struct SqlxResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<SqlxResourceProviderInner>>,
}

pub struct SqlxResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
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
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        sqlx_url: &str,
        workflow_id: &str,
    ) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let mut telemetry_handle = telemetry_handle;
        let sqlx_url = sqlx_url.to_string();

        let workflow_id = workflow_id.to_string();
        let handle = tokio::spawn(async move {
            loop {
                let workflow_id = workflow_id.clone();

                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                    created,
                    metadata,
                } = dataplane_handle.receive_next().await;
                let started = crate::resources::observe_transfer(created, &mut telemetry_handle);

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
                    log::info!("sqlx Database exists");
                }

                let db = SqlitePool::connect(&sqlx_url).await.unwrap();

                let response = sqlx::query(
                    "CREATE TABLE IF NOT EXISTS WorkflowState (
                    id VARCHAR(255) PRIMARY KEY,
                    metadata JSONB NOT NULL);",
                )
                .execute(&db)
                .await
                .unwrap();

                log::info!("create table in sql {:?}", response);

                if message_data.to_string().contains("SELECT") {
                    let result: sqlx::Result<WorkflowState, sqlx::Error> =
                        sqlx::query_as(message_data.as_str())
                            .bind(workflow_id)
                            .fetch_one(&db)
                            .await;

                    match result {
                        Ok(response) => {
                            log::info!("Response from database: {response:?}");
                            if need_reply {
                                dataplane_handle
                                    .reply(
                                        source_id,
                                        channel_id,
                                        edgeless_dataplane::core::CallRet::Reply(
                                            serde_json::to_string(&response).unwrap_or_default(),
                                        ),
                                        &metadata,
                                    )
                                    .await;
                            }
                        }
                        Err(e) => {
                            log::info!("Response from database: {:?}", e.to_string());
                            dataplane_handle
                                .reply(
                                    source_id,
                                    channel_id,
                                    edgeless_dataplane::core::CallRet::Reply(e.to_string()),
                                    &metadata,
                                )
                                .await;
                        }
                    }
                } else if message_data.to_string().contains("INSERT")
                    || message_data.to_string().contains("UPDATE")
                    || message_data.to_string().contains("DELETE")
                {
                    let result = sqlx::query(message_data.as_str())
                        .bind(workflow_id)
                        .execute(&db)
                        .await;
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
                                    .reply(
                                        source_id,
                                        channel_id,
                                        edgeless_dataplane::core::CallRet::Reply(res),
                                        &metadata,
                                    )
                                    .await;
                            }
                        }

                        Err(e) => {
                            log::info!("Error from state management: {:?}", e);
                            dataplane_handle
                                .reply(
                                    source_id,
                                    channel_id,
                                    edgeless_dataplane::core::CallRet::Reply(e.to_string()),
                                    &metadata,
                                )
                                .await;
                        }
                    }
                } else {
                    log::info!("Unknow operation in state management");
                    dataplane_handle
                        .reply(
                            source_id,
                            channel_id,
                            edgeless_dataplane::core::CallRet::Err,
                            &metadata,
                        )
                        .await;
                };

                db.close().await;
                crate::resources::observe_execution(started, &mut telemetry_handle, need_reply);
            }
        });

        Ok(Self {
            join_handle: handle,
        })
    }
}

impl SqlxResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(SqlxResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                telemetry_handle,
                instances: std::collections::HashMap::<
                    edgeless_api::function_instance::InstanceId,
                    SqlxResource,
                >::new(),
            })),
        }
    }
}

#[async_trait::async_trait]
impl
    edgeless_api::resource_configuration::ResourceConfigurationAPI<
        edgeless_api::function_instance::InstanceId,
    > for SqlxResourceProvider
{
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<
        edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>,
    > {
        if let (Some(url), workflow_id) = (
            instance_specification.configuration.get("url"),
            instance_specification.workflow_id,
        ) {
            let mut lck = self.inner.lock().await;
            let new_id =
                edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;
            let telemetry_handle = lck
                .telemetry_handle
                .fork(std::collections::BTreeMap::from([(
                    "FUNCTION_ID".to_string(),
                    new_id.function_id.to_string(),
                )]));

            match SqlxResource::new(dataplane_handle, telemetry_handle, url, &workflow_id).await {
                Ok(resource) => {
                    lck.instances.insert(new_id, resource);
                    return Ok(edgeless_api::common::StartComponentResponse::InstanceId(
                        new_id,
                    ));
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

    async fn stop(
        &mut self,
        resource_id: edgeless_api::function_instance::InstanceId,
    ) -> anyhow::Result<()> {
        self.inner.lock().await.instances.remove(&resource_id);
        Ok(())
    }

    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        // the resource has no channels: nothing to be patched
        Ok(())
    }
}

#[derive(Clone, FromRow, Debug, serde::Deserialize, serde::Serialize)]
struct WorkflowState {
    id: String,
    metadata: serde_json::Value,
}

impl std::fmt::Display for WorkflowState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "id: {}, metadata: {}", self.id, self.metadata)
    }
}
