// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

extern crate redis;
use futures::{SinkExt, StreamExt};
use redis::Commands;

pub struct MetricsCollectorResourceSpec {}

impl super::resource_provider_specs::ResourceProviderSpecs for MetricsCollectorResourceSpec {
    fn class_type(&self) -> String {
        String::from("metrics-collector")
    }

    fn outputs(&self) -> Vec<String> {
        vec![]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([
            (
                String::from("alpha"),
                String::from("Coefficient to filter averages with exponential smoothing"),
            ),
            (String::from("wf_name"), String::from("Workflow identifier used to save stats")),
        ])
    }

    fn version(&self) -> String {
        String::from("1.1")
    }
}

#[derive(Clone)]
pub struct MetricsCollectorResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<MetricsCollectorResourceProviderInner>>,
}

enum Event {
    /// A new workflow-level transaction begins, with given identifier.
    WorkflowBegin(u64),
    /// A workflow-level transaction ends, with given identifier.
    WorkflowEnd(u64),
    /// A new function-level transaction begins, with given identifier.
    FunctionBegin(u64),
    /// A function-level transaction ends, with given identifier.
    FunctionEnd(u64),
    /// A new epoch begins, with given warm-up period, in ms.
    Reset(u64),
}

impl Event {
    fn new(value: &str) -> anyhow::Result<Self> {
        let tokens: Vec<&str> = value.split(':').collect();
        if tokens.len() == 2 && tokens[0] == "reset" {
            let warmup = match tokens[1].parse::<u64>() {
                Ok(val) => val,
                Err(err) => anyhow::bail!("warm-up period parse error: {}", err),
            };
            return Ok(Event::Reset(warmup));
        }
        anyhow::ensure!(tokens.len() == 3, "invalid number of tokens, expected 3 found {}", tokens.len());
        let transaction = match tokens[2].parse::<u64>() {
            Ok(val) => val,
            Err(err) => anyhow::bail!("transaction parse error: {}", err),
        };
        if tokens[0] == "workflow" {
            if tokens[1] == "begin" {
                Ok(Event::WorkflowBegin(transaction))
            } else if tokens[1] == "end" {
                return Ok(Event::WorkflowEnd(transaction));
            } else {
                anyhow::bail!("invalid workflow command: {}", tokens[1]);
            }
        } else if tokens[0] == "function" {
            if tokens[1] == "begin" {
                return Ok(Event::FunctionBegin(transaction));
            } else if tokens[1] == "end" {
                return Ok(Event::FunctionEnd(transaction));
            } else {
                anyhow::bail!("invalid workflow command: {}", tokens[1]);
            }
        } else {
            anyhow::bail!("invalid event element: {}", tokens[1]);
        }
    }

    fn initial(&self) -> &str {
        match self {
            Event::WorkflowBegin(_) | Event::WorkflowEnd(_) => "W",
            Event::FunctionBegin(_) | Event::FunctionEnd(_) => "F",
            _ => "",
        }
    }

    fn full(&self) -> &str {
        match self {
            Event::WorkflowBegin(_) | Event::WorkflowEnd(_) => "workflow",
            Event::FunctionBegin(_) | Event::FunctionEnd(_) => "function",
            _ => "",
        }
    }

    fn transaction(&self) -> u64 {
        match self {
            Event::WorkflowBegin(transaction)
            | Event::WorkflowEnd(transaction)
            | Event::FunctionBegin(transaction)
            | Event::FunctionEnd(transaction) => *transaction,
            _ => 0,
        }
    }

    fn workflow(&self) -> bool {
        matches!(self, Event::WorkflowBegin(_) | Event::WorkflowEnd(_))
    }

    fn begin(&self) -> bool {
        matches!(self, Event::WorkflowBegin(_) | Event::FunctionBegin(_))
    }
}

enum RedisCommand {
    // key, value, timestamp
    Push(String, i64, std::time::SystemTime),
    // key, value
    Set(String, f64),
    // warmup period (in ms)
    Reset(u64),
}

impl RedisCommand {
    fn reset(&self) -> bool {
        matches!(self, RedisCommand::Reset(_))
    }

    fn timestamp(instant: &std::time::SystemTime) -> String {
        let duration = instant.duration_since(std::time::UNIX_EPOCH).unwrap();
        format!("{}.{}", duration.as_secs(), duration.subsec_millis())
    }
}

impl std::fmt::Display for RedisCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RedisCommand::Push(key, value, instant) => write!(f, "({},{},{})", key, value, RedisCommand::timestamp(instant)),
            RedisCommand::Set(key, value) => write!(f, "({},{})", key, value),
            RedisCommand::Reset(warmup) => write!(f, "(reset {} ms)", warmup),
        }
    }
}

pub struct MetricsCollectorResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, MetricsCollectorResource>,
    sender: futures::channel::mpsc::UnboundedSender<RedisCommand>,
    _handle: tokio::task::JoinHandle<()>,
}

pub struct MetricsCollectorResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for MetricsCollectorResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl MetricsCollectorResource {
    // alpha is the smoothing factor of the EWMA of samples.
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        alpha: f64,
        wf_name: String,
        sender: futures::channel::mpsc::UnboundedSender<RedisCommand>,
    ) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let mut sender = sender;
        let mut telemetry_handle = telemetry_handle;

        let handle = tokio::spawn(async move {
            let mut timestamps = std::collections::HashMap::new();
            let mut averages = std::collections::HashMap::new();
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                    created,
                } = dataplane_handle.receive_next().await;
                let started = crate::resources::observe_transfer(created, &mut telemetry_handle);

                let mut need_reply = false;
                let message_data = match message {
                    edgeless_dataplane::core::Message::Call(data) => {
                        need_reply = true;
                        data
                    }
                    edgeless_dataplane::core::Message::Cast(data) => data,
                    _ => {
                        continue;
                    }
                };

                match Event::new(&message_data) {
                    Ok(event) => {
                        if let Event::Reset(warmup) = event {
                            let _ = sender.send(RedisCommand::Reset(warmup)).await;
                        } else if event.workflow() && wf_name.is_empty() {
                            // Skip workflow events with empty workflow name.
                            continue;
                        } else {
                            let id = match event.workflow() {
                                true => wf_name.clone(),
                                false => source_id.function_id.to_string(),
                            };
                            let key = format!("{}:{}:{}", event.initial(), id, event.transaction());
                            let avg_key = format!("{}:{}", event.initial(), id);
                            if event.begin() {
                                timestamps.insert(key, std::time::Instant::now());
                            } else if let Some(ts) = timestamps.remove(&key) {
                                let current = ts.elapsed().as_millis() as i64;
                                let _ = sender
                                    .send(RedisCommand::Push(
                                        format!("{}:{}:samples", event.full(), id),
                                        current,
                                        std::time::SystemTime::now(),
                                    ))
                                    .await;
                                let average = match averages.get(&avg_key) {
                                    Some(prev_value) => current as f64 * alpha + (1.0_f64 - alpha) * prev_value,
                                    None => current as f64,
                                };
                                averages.insert(avg_key, average);
                                let _ = sender.send(RedisCommand::Set(format!("{}:{}:average", event.full(), id), average)).await;
                            }
                        }
                    }
                    Err(err) => log::warn!("invalid metrics-collector event received: {}", err),
                }

                if need_reply {
                    dataplane_handle
                        .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply("".to_string()))
                        .await;
                }

                crate::resources::observe_execution(started, &mut telemetry_handle, need_reply);
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl MetricsCollectorResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
        redis_connection: redis::Connection,
    ) -> Self {
        // Create a channel for:
        // - single receiver: the loop in the task below
        // - multiple senders: the resource instances that will be created
        //   at run-time
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let mut receiver: futures::channel::mpsc::UnboundedReceiver<RedisCommand> = receiver;
        let mut redis_connection = redis_connection;
        let _handle = tokio::spawn(async move {
            let mut keys = std::collections::HashSet::new();
            let mut ts = std::time::Instant::now();
            let mut warmup = std::time::Duration::from_secs(0);
            while let Some(command) = receiver.next().await {
                // If this is a non-reset command we check if we are still in
                // the warm-up period of this epoch, in which case we skip this
                // command.
                if !command.reset() && ts.elapsed() <= warmup {
                    continue;
                }

                let res = match &command {
                    RedisCommand::Push(key, value, instant) => {
                        keys.insert(key.to_string());
                        redis_connection
                            .rpush::<&str, String, usize>(key, format!("{},{}", *value, RedisCommand::timestamp(instant)))
                            .err()
                    }
                    RedisCommand::Set(key, value) => {
                        keys.insert(key.to_string());
                        redis_connection.set::<&str, f64, String>(key, *value).err()
                    }
                    RedisCommand::Reset(new_warmup) => {
                        log::info!("resetting the metrics, a new epoch starts with warm-up period {} ms", new_warmup);

                        // Clean the Redis database from all the keys added.
                        for key in keys.drain() {
                            let _ = redis_connection.del::<String, usize>(key);
                        }

                        // Restart the timer for detecting the warmup.
                        ts = std::time::Instant::now();
                        warmup = std::time::Duration::from_millis(*new_warmup);
                        None
                    }
                };
                if let Some(err) = res {
                    log::error!("Redis error when setting {}: {}", command, err);
                }
            }
        });
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(MetricsCollectorResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                telemetry_handle,
                instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, MetricsCollectorResource>::new(),
                sender,
                _handle,
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>
    for MetricsCollectorResourceProvider
{
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let mut lck = self.inner.lock().await;
        let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
        let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;

        // Read configuration
        let alpha = instance_specification
            .configuration
            .get("alpha")
            .unwrap_or(&"".to_string())
            .parse::<f64>()
            .unwrap_or(0.9_f64);

        let wf_name = instance_specification.configuration.get("wf_name").unwrap_or(&"".to_string()).clone();

        match MetricsCollectorResource::new(
            dataplane_handle,
            lck.telemetry_handle.fork(std::collections::BTreeMap::from([(
                "FUNCTION_ID".to_string(),
                new_id.function_id.to_string(),
            )])),
            alpha,
            wf_name,
            lck.sender.clone(),
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

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.inner.lock().await.instances.remove(&resource_id);
        Ok(())
    }

    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        // the resource has no channels: nothing to be patched
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn metrics_collector_commands_race() {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let mut sender = sender;
        let mut sender_push = sender.clone();
        let handle_sender_push = tokio::spawn(async move {
            println!("started sender-push");
            for cnt in 0..10 {
                let _ = sender_push
                    .send(RedisCommand::Push("my-push".to_string(), cnt, std::time::SystemTime::now()))
                    .await;
            }
        });
        let mut sender_set = sender.clone();
        let handle_sender_set = tokio::spawn(async move {
            println!("started sender-set");
            for cnt in 0..10 {
                let _ = sender_set.send(RedisCommand::Set("my-set".to_string(), cnt as f64)).await;
            }
        });

        let mut receiver: futures::channel::mpsc::UnboundedReceiver<RedisCommand> = receiver;
        let handle_receiver = tokio::spawn(async move {
            println!("started receiver");
            while let Some(command) = receiver.next().await {
                println!("{}", command);
            }
        });
        let _ = sender.close().await;

        println!("start");
        let _ = futures::join!(handle_receiver, handle_sender_set, handle_sender_push);
    }
}
