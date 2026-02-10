// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use base64::Engine;

pub struct FilePusherResourceSpec {}

impl super::resource_provider_specs::ResourceProviderSpecs for FilePusherResourceSpec {
    fn class_type(&self) -> String {
        String::from("file-pusher")
    }

    fn description(&self) -> String {
        r"Read file content from a directory on the local filesystem and cast periodically to the output channel in round-robin. Resources are pure triggers, i.e., they do not handle events".to_string()
    }

    fn outputs(&self) -> Vec<String> {
        vec![String::from("out")]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([
            (String::from("period-ms"), String::from("The interval at which files are pushed, in ms")),
            (String::from("encode-base64"), String::from("Encode the file content in base64")),
        ])
    }

    fn version(&self) -> String {
        String::from("1.0")
    }
}

#[derive(Clone)]
pub struct FilePusherResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<FilePusherResourceProviderInner>>,
}

struct FilePusherResourceProviderInner {
    node_id: edgeless_api::function_instance::NodeId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    instances: std::collections::HashMap<edgeless_api::function_instance::ComponentId, FilePusherResource>,
    files: Vec<Vec<u8>>,
}

pub struct FilePusherResource {
    join_handle: tokio::task::JoinHandle<()>,
    target: Option<edgeless_api::function_instance::InstanceId>,
}

impl Drop for FilePusherResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl FilePusherResource {
    async fn new(
        period_ms: u64,
        encode_base64: bool,
        num_files: usize,
        self_function_id: edgeless_api::function_instance::ComponentId,
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        inner: std::sync::Arc<tokio::sync::Mutex<FilePusherResourceProviderInner>>,
    ) -> Self {
        let mut dataplane_handle = dataplane_handle;
        let mut telemetry_handle = telemetry_handle;

        let handle = tokio::spawn(async move {
            let mut cur = 0_usize;
            loop {
                let started = chrono::Utc::now();

                let inner = inner.lock().await;

                let msg = if num_files == 0 {
                    String::default()
                } else {
                    if encode_base64 {
                        base64::engine::general_purpose::STANDARD.encode(inner.files[cur].clone())
                    } else {
                        String::from_utf8(inner.files[cur].to_vec()).unwrap_or_default()
                    }
                };

                if let Some(instance) = inner.instances.get(&self_function_id) {
                    if let Some(instance_id) = instance.target {
                        dataplane_handle
                            .send(instance_id, msg, &edgeless_api::function_instance::EventMetadata::empty_new_root())
                            .await;
                    }
                }

                // Move to the next file. Wrap-around, if needed.
                cur += 1;
                if cur >= num_files {
                    cur = 0;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(period_ms)).await;

                crate::resources::observe_execution(started, &mut telemetry_handle, true);
            }
        });

        Self {
            join_handle: handle,
            target: None,
        }
    }
}

impl FilePusherResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        node_id: edgeless_api::function_instance::NodeId,
        directory: &str,
    ) -> Self {
        // Read all the files in the input directory, sort them in
        // lexycographic order.
        let mut files = vec![];
        match std::fs::read_dir(directory) {
            Ok(paths) => {
                let mut filenames = vec![];
                for path in paths {
                    if let Ok(path) = path {
                        if let Some(filename) = path.path().as_os_str().to_str() {
                            filenames.push(filename.to_string());
                        }
                    }
                }
                filenames.sort();
                for filename in filenames {
                    if let Ok(content) = std::fs::read(filename) {
                        files.push(content);
                    }
                }
            }
            Err(err) => log::error!("could not read files for file-pusher from directory '{directory}': {err}"),
        }

        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(FilePusherResourceProviderInner {
                node_id,
                dataplane_provider,
                telemetry_handle,
                instances: std::collections::HashMap::new(),
                files,
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for FilePusherResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let period_ms = instance_specification
            .configuration
            .get("period-ms")
            .unwrap_or(&String::from("1000"))
            .parse::<u64>()
            .unwrap_or(1000);
        let encode_base64 = instance_specification
            .configuration
            .get("encode-base64")
            .unwrap_or(&String::from("false"))
            .eq_ignore_ascii_case("true");

        let mut lck = self.inner.lock().await;
        let num_files = lck.files.len();

        let new_id = edgeless_api::function_instance::InstanceId::new(lck.node_id);
        let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;

        let telemetry_handle = lck.telemetry_handle.fork(std::collections::BTreeMap::from([(
            "FUNCTION_ID".to_string(),
            new_id.function_id.to_string(),
        )]));
        lck.instances.insert(
            new_id.function_id,
            FilePusherResource::new(
                period_ms,
                encode_base64,
                num_files,
                new_id.function_id,
                dataplane_handle,
                telemetry_handle,
                self.inner.clone(),
            )
            .await,
        );

        Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
    }

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.inner.lock().await.instances.remove(&resource_id.function_id);
        Ok(())
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        let mut lck = self.inner.lock().await;
        if let Some(instance) = lck.instances.get_mut(&update.function_id) {
            if let Some(target) = update.output_mapping.get("out") {
                instance.target = Some(*target);
            } else {
                instance.target = None;
            }
        } else {
            anyhow::bail!("Patching a non-existing resource: {}", update.function_id);
        }

        Ok(())
    }
}
