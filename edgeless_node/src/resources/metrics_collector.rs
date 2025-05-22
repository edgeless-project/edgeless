// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

extern crate redis;
use futures::{SinkExt, StreamExt};
use redis::Commands;
use std::time::Duration;

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
    /// A new workflow-level transaction begins, with given: (timestamp, workflow_id).
    WorkflowBegin(u64, u64),
    /// A workflow-level transaction ends, with given: (timestamp, workflow_id).
    WorkflowEnd(u64, u64),
    /// A new function-level transaction begins, with given: (timestamp, workflow_id, function_id).
    FunctionBegin(u64, u64, u64),
    /// A function-level transaction ends, with given: (timestamp, workflow_id, function_id).
    FunctionEnd(u64, u64, u64),
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
        } else if tokens.len() == 4 && tokens[0] == "workflow" {
            let timestamp = match tokens[2].parse::<u64>() {
                Ok(val) => val,
                Err(err) => anyhow::bail!("timestamp parse error: {}", err),
            };
            let workflow_id = match tokens[3].parse::<u64>() {
                Ok(val) => val,
                Err(err) => anyhow::bail!("transaction parse error: {}", err),
            };
            if tokens[1] == "begin" {
                return Ok(Event::WorkflowBegin(timestamp, workflow_id));
            } else if tokens[1] == "end" {
                return Ok(Event::WorkflowEnd(timestamp, workflow_id));
            } else {
                anyhow::bail!("invalid workflow command: {}", tokens[1]);
            }
        } else if tokens.len() == 5 && tokens[0] == "function" {
            let timestamp = match tokens[2].parse::<u64>() {
                Ok(val) => val,
                Err(err) => anyhow::bail!("timestamp parse error: {}", err),
            };
            let workflow_id = match tokens[3].parse::<u64>() {
                Ok(val) => val,
                Err(err) => anyhow::bail!("workflow_id parse error: {}", err),
            };
            let function_id = match tokens[4].parse::<u64>() {
                Ok(val) => val,
                Err(err) => anyhow::bail!("function_id parse error: {}", err),
            };
            if tokens[1] == "begin" {
                return Ok(Event::FunctionBegin(timestamp, workflow_id, function_id));
            } else if tokens[1] == "end" {
                return Ok(Event::FunctionEnd(timestamp, workflow_id, function_id));
            } else {
                anyhow::bail!("invalid workflow command: {}", tokens[1]);
            }
        } else {
            anyhow::bail!("invalid event element: {}", tokens[1]);
        }
    }

    fn initial(&self) -> &str {
        match self {
            Event::WorkflowBegin(_, _) | Event::WorkflowEnd(_, _) => "W",
            Event::FunctionBegin(_, _, _) | Event::FunctionEnd(_, _, _) => "F",
            _ => "",
        }
    }

    fn full(&self) -> &str {
        match self {
            Event::WorkflowBegin(_, _) | Event::WorkflowEnd(_, _) => "workflow",
            Event::FunctionBegin(_, _, _) | Event::FunctionEnd(_, _, _) => "function",
            _ => "",
        }
    }

    fn transaction(&self) -> u64 {
        match self {
            Event::WorkflowBegin(_, transaction)
            | Event::WorkflowEnd(_, transaction)
            | Event::FunctionBegin(_, _, transaction)
            | Event::FunctionEnd(_, _, transaction) => *transaction,
            _ => 0,
        }
    }

    fn timestamp(&self) -> u64 {
        match self {
            Event::WorkflowBegin(timestamp, _)
            | Event::WorkflowEnd(timestamp, _)
            | Event::FunctionBegin(timestamp, _, _)
            | Event::FunctionEnd(timestamp, _, _) => *timestamp,
            _ => 0,
        }
    }

    fn workflow(&self) -> bool {
        matches!(self, Event::WorkflowBegin(_, _) | Event::WorkflowEnd(_, _))
    }

    fn begin(&self) -> bool {
        matches!(self, Event::WorkflowBegin(_, _) | Event::FunctionBegin(_, _, _))
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
        telemetry_handle: std::sync::Arc<tokio::sync::Mutex<Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>>>,
        alpha: f64,
        wf_name: String,
        sender: std::sync::Arc<tokio::sync::Mutex<futures::channel::mpsc::UnboundedSender<RedisCommand>>>,
    ) -> anyhow::Result<Self> {
        let dataplane_handle = dataplane_handle;
        let sender = sender;
        // let telemetry_handle = telemetry_handle;

        let handle = tokio::spawn(async move {
            let timestamps = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
            let averages = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                    created,
                } = dataplane_handle.clone().receive_next().await;
                // TODO: fix
                // let started = crate::resources::observe_transfer(created, &mut telemetry_handle.lock().await);

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

                // the handling of a new event should be non-blocking to avoid
                // side effect from collection
                // first shadow the variables with their copies
                let mut dataplane_handle = dataplane_handle.clone();
                let sender = sender.clone();
                let wf_name = wf_name.clone();
                let timestamps = timestamps.clone();
                let averages = averages.clone();

                // run in the background to avoid problems
                let handle = tokio::spawn(async move {
                    let mut sender_guard = sender.lock().await;
                    match Event::new(&message_data) {
                        Ok(event) => {
                            if let Event::Reset(warmup) = event {
                                let _ = sender_guard.send(RedisCommand::Reset(warmup)).await;
                            } else if event.workflow() && wf_name.is_empty() {
                                ()
                            } else {
                                // workflow or function event
                                let id = match event.workflow() {
                                    true => wf_name,
                                    false => source_id.function_id.to_string(),
                                };
                                let key = format!("{}:{}:{}", event.initial(), id, event.transaction());
                                let avg_key = format!("{}:{}", event.initial(), id);
                                let timestamp = event.timestamp();
                                if event.begin() {
                                    timestamps.lock().await.insert(key, timestamp);
                                } else if let Some(begin_timestamp) = timestamps.lock().await.remove(&key) {
                                    // calculate the time difference
                                    let current: i64 = (timestamp - begin_timestamp) as i64;
                                    let _ = sender_guard
                                        .send(RedisCommand::Push(
                                            format!("{}:{}:samples", event.full(), id),
                                            current,
                                            std::time::SystemTime::now(),
                                        ))
                                        .await;
                                    let average = match averages.lock().await.get(&avg_key) {
                                        Some(prev_value) => current as f64 * alpha + (1.0_f64 - alpha) * prev_value,
                                        None => current as f64,
                                    };
                                    averages.lock().await.insert(avg_key, average);
                                    let _ = sender_guard
                                        .send(RedisCommand::Set(format!("{}:{}:average", event.full(), id), average))
                                        .await;
                                } else {
                                    // this only occurs if for some reason the
                                    // workflow / function end timestamp is
                                    // received before the begin timestamp.
                                    panic!("should never happen");
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

                    // TODO: fix at some point
                    // crate::resources::observe_execution(started, &mut telemetry_handle.lock().await.as_ref(), need_reply);
                });
                // warn the user if a metric could not be persisted
                let _ = tokio::time::timeout(Duration::from_millis(500), handle).await.unwrap_or_else(|_| {
                    log::warn!("metric could not be persisted");
                    Ok(())
                });
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
            std::sync::Arc::new(tokio::sync::Mutex::new(lck.telemetry_handle.fork(std::collections::BTreeMap::from([(
                "FUNCTION_ID".to_string(),
                new_id.function_id.to_string(),
            )])))),
            alpha,
            wf_name,
            std::sync::Arc::new(tokio::sync::Mutex::new(lck.sender.clone())),
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
