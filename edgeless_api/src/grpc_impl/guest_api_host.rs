// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub struct GuestAPIHostClient {
    client: crate::grpc_impl::api::guest_api_host_client::GuestApiHostClient<tonic::transport::Channel>,
}

pub struct GuestAPIHostService {
    pub guest_api_host: tokio::sync::Mutex<Box<dyn crate::guest_api_host::GuestAPIHost>>,
}

impl GuestAPIHostClient {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        loop {
            match crate::grpc_impl::api::guest_api_host_client::GuestApiHostClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Ok(Self { client });
                }
                Err(err) => match retry_interval {
                    Some(val) => tokio::time::sleep(tokio::time::Duration::from_secs(val)).await,
                    None => {
                        return Err(anyhow::anyhow!("Error when connecting to {}: {}", server_addr, err));
                    }
                },
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::guest_api_host::GuestAPIHost for GuestAPIHostClient {
    async fn cast(&mut self, event: crate::guest_api_host::OutputEventData) -> anyhow::Result<()> {
        match self.client.cast(tonic::Request::new(serialize_output_event_data(&event))).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Communication error while casting an event: {}", err.to_string())),
        }
    }
    async fn cast_raw(&mut self, event: crate::guest_api_host::OutputEventDataRaw) -> anyhow::Result<()> {
        match self.client.cast_raw(tonic::Request::new(serialize_output_event_data_raw(&event))).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Communication error while raw-casting an event: {}", err.to_string())),
        }
    }
    async fn call(&mut self, event: crate::guest_api_host::OutputEventData) -> anyhow::Result<crate::guest_api_function::CallReturn> {
        match self.client.call(tonic::Request::new(serialize_output_event_data(&event))).await {
            Ok(msg) => crate::grpc_impl::guest_api_function::parse_call_return(&msg.into_inner()),
            Err(err) => Err(anyhow::anyhow!("Communication error while calling a function: {}", err.to_string())),
        }
    }
    async fn call_raw(&mut self, event: crate::guest_api_host::OutputEventDataRaw) -> anyhow::Result<crate::guest_api_function::CallReturn> {
        match self.client.call_raw(tonic::Request::new(serialize_output_event_data_raw(&event))).await {
            Ok(msg) => crate::grpc_impl::guest_api_function::parse_call_return(&msg.into_inner()),
            Err(err) => Err(anyhow::anyhow!("Communication error while raw-calling a function: {}", err.to_string())),
        }
    }
    async fn telemetry_log(&mut self, event: crate::guest_api_host::TelemetryLogEvent) -> anyhow::Result<()> {
        match self
            .client
            .telemetry_log(tonic::Request::new(serialize_telemetry_log_event(&event)))
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while emitting a telemetry log event: {}",
                err.to_string()
            )),
        }
    }
    async fn slf(&mut self) -> anyhow::Result<edgeless_api_core::instance_id::InstanceId> {
        match self.client.slf(tonic::Request::new(())).await {
            Ok(msg) => crate::grpc_impl::common::CommonConverters::parse_instance_id(&msg.into_inner()),
            Err(err) => Err(anyhow::anyhow!("Communication error while casting an event: {}", err.to_string())),
        }
    }
    async fn delayed_cast(&mut self, event: crate::guest_api_host::DelayedEventData) -> anyhow::Result<()> {
        match self.client.delayed_cast(tonic::Request::new(serialize_delayed_event_data(&event))).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Communication error while casting a delayed event: {}", err.to_string())),
        }
    }
    async fn sync(&mut self, event: crate::guest_api_host::SyncData) -> anyhow::Result<()> {
        match self.client.sync(tonic::Request::new(serialize_sync_data(&event))).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Communication error while synchronizing data: {}", err.to_string())),
        }
    }
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::guest_api_host_server::GuestApiHost for GuestAPIHostService {
    async fn cast(&self, event: tonic::Request<crate::grpc_impl::api::OutputEventData>) -> Result<tonic::Response<()>, tonic::Status> {
        let parsed_request = match parse_output_event_data(&event.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing an OutputEventData message: {}",
                    err
                )));
            }
        };
        match self.guest_api_host.lock().await.cast(parsed_request).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when casting an event: {}", err))),
        }
    }

    async fn cast_raw(&self, event: tonic::Request<crate::grpc_impl::api::OutputEventDataRaw>) -> Result<tonic::Response<()>, tonic::Status> {
        let parsed_request = match parse_output_event_data_raw(&event.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing an OutputEventDataRaw message: {}",
                    err
                )));
            }
        };
        match self.guest_api_host.lock().await.cast_raw(parsed_request).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when raw-casting an event: {}", err))),
        }
    }

    async fn call(
        &self,
        event: tonic::Request<crate::grpc_impl::api::OutputEventData>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::CallReturn>, tonic::Status> {
        let parsed_request = match parse_output_event_data(&event.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing an OutputEventData message: {}",
                    err
                )));
            }
        };
        match self.guest_api_host.lock().await.call(parsed_request).await {
            Ok(msg) => Ok(tonic::Response::new(crate::grpc_impl::guest_api_function::serialize_call_return(&msg))),
            Err(err) => Err(tonic::Status::internal(format!("Error when calling a function: {}", err))),
        }
    }

    async fn call_raw(
        &self,
        event: tonic::Request<crate::grpc_impl::api::OutputEventDataRaw>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::CallReturn>, tonic::Status> {
        let parsed_request = match parse_output_event_data_raw(&event.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing an OutputEventDataRaw message: {}",
                    err
                )));
            }
        };
        match self.guest_api_host.lock().await.call_raw(parsed_request).await {
            Ok(msg) => Ok(tonic::Response::new(crate::grpc_impl::guest_api_function::serialize_call_return(&msg))),
            Err(err) => Err(tonic::Status::internal(format!("Error when raw-calling a function: {}", err))),
        }
    }

    async fn telemetry_log(&self, event: tonic::Request<crate::grpc_impl::api::TelemetryLogEvent>) -> Result<tonic::Response<()>, tonic::Status> {
        let parsed_request = match parse_telemetry_log_event(&event.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing a TelemetryLogEvent message: {}",
                    err
                )));
            }
        };
        match self.guest_api_host.lock().await.telemetry_log(parsed_request).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when emitting a telemetry log event: {}", err))),
        }
    }

    async fn slf(&self, _request: tonic::Request<()>) -> Result<tonic::Response<crate::grpc_impl::api::InstanceId>, tonic::Status> {
        match self.guest_api_host.lock().await.slf().await {
            Ok(msg) => Ok(tonic::Response::new(crate::grpc_impl::common::CommonConverters::serialize_instance_id(
                &msg,
            ))),
            Err(err) => Err(tonic::Status::internal(format!("Error when raw-casting an event: {}", err))),
        }
    }

    async fn delayed_cast(&self, event: tonic::Request<crate::grpc_impl::api::DelayedEventData>) -> Result<tonic::Response<()>, tonic::Status> {
        let parsed_request = match parse_delayed_event_data(&event.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing a DelayedEventData message: {}",
                    err
                )));
            }
        };
        match self.guest_api_host.lock().await.delayed_cast(parsed_request).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when casting a delayed event: {}", err))),
        }
    }

    async fn sync(&self, event: tonic::Request<crate::grpc_impl::api::SyncData>) -> Result<tonic::Response<()>, tonic::Status> {
        let parsed_request = match parse_sync_data(&event.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!("Error when parsing a SyncData message: {}", err)));
            }
        };
        match self.guest_api_host.lock().await.sync(parsed_request).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when synchronizing: {}", err))),
        }
    }
}

pub fn parse_output_event_data(api_instance: &crate::grpc_impl::api::OutputEventData) -> anyhow::Result<crate::guest_api_host::OutputEventData> {
    Ok(crate::guest_api_host::OutputEventData {
        originator: match &api_instance.originator {
            Some(instance_id) => match crate::grpc_impl::common::CommonConverters::parse_instance_id(instance_id) {
                Ok(originator) => originator,
                Err(err) => return Err(anyhow::anyhow!("invalid originator field: {}", err)),
            },
            None => return Err(anyhow::anyhow!("missing originator field")),
        },
        alias: api_instance.alias.clone(),
        msg: api_instance.msg.clone(),
    })
}

pub fn parse_output_event_data_raw(
    api_instance: &crate::grpc_impl::api::OutputEventDataRaw,
) -> anyhow::Result<crate::guest_api_host::OutputEventDataRaw> {
    match &api_instance.dst {
        Some(instance_id) => match crate::grpc_impl::common::CommonConverters::parse_instance_id(&instance_id) {
            Ok(dst) => Ok(crate::guest_api_host::OutputEventDataRaw {
                originator: match &api_instance.originator {
                    Some(instance_id) => match crate::grpc_impl::common::CommonConverters::parse_instance_id(&instance_id) {
                        Ok(originator) => originator,
                        Err(err) => return Err(anyhow::anyhow!("invalid originator field: {}", err)),
                    },
                    None => return Err(anyhow::anyhow!("missing originator field")),
                },
                dst,
                msg: api_instance.msg.clone(),
            }),
            Err(err) => Err(err),
        },
        None => Err(anyhow::anyhow!("dst is missing")),
    }
}

pub fn parse_telemetry_log_event(
    api_instance: &crate::grpc_impl::api::TelemetryLogEvent,
) -> anyhow::Result<crate::guest_api_host::TelemetryLogEvent> {
    Ok(crate::guest_api_host::TelemetryLogEvent {
        originator: match &api_instance.originator {
            Some(instance_id) => match crate::grpc_impl::common::CommonConverters::parse_instance_id(&instance_id) {
                Ok(originator) => originator,
                Err(err) => return Err(anyhow::anyhow!("invalid originator field: {}", err)),
            },
            None => return Err(anyhow::anyhow!("missing originator field")),
        },
        log_level: match api_instance.log_level {
            x if x == crate::grpc_impl::api::TelemetryLogLevel::LogError as i32 => crate::guest_api_host::TelemetryLogLevel::Error,
            x if x == crate::grpc_impl::api::TelemetryLogLevel::LogWarn as i32 => crate::guest_api_host::TelemetryLogLevel::Warn,
            x if x == crate::grpc_impl::api::TelemetryLogLevel::LogInfo as i32 => crate::guest_api_host::TelemetryLogLevel::Info,
            x if x == crate::grpc_impl::api::TelemetryLogLevel::LogDebug as i32 => crate::guest_api_host::TelemetryLogLevel::Debug,
            x if x == crate::grpc_impl::api::TelemetryLogLevel::LogTrace as i32 => crate::guest_api_host::TelemetryLogLevel::Trace,
            x => return Err(anyhow::anyhow!("invalid telemetry event log level: {}", x as i32)),
        },
        target: api_instance.target.clone(),
        msg: api_instance.msg.clone(),
    })
}

pub fn parse_delayed_event_data(api_instance: &crate::grpc_impl::api::DelayedEventData) -> anyhow::Result<crate::guest_api_host::DelayedEventData> {
    Ok(crate::guest_api_host::DelayedEventData {
        originator: match &api_instance.originator {
            Some(instance_id) => match crate::grpc_impl::common::CommonConverters::parse_instance_id(&instance_id) {
                Ok(originator) => originator,
                Err(err) => return Err(anyhow::anyhow!("invalid originator field: {}", err)),
            },
            None => return Err(anyhow::anyhow!("missing originator field")),
        },
        delay: api_instance.delay,
        alias: api_instance.alias.clone(),
        msg: api_instance.msg.clone(),
    })
}

pub fn parse_sync_data(api_instance: &crate::grpc_impl::api::SyncData) -> anyhow::Result<crate::guest_api_host::SyncData> {
    Ok(crate::guest_api_host::SyncData {
        originator: match &api_instance.originator {
            Some(instance_id) => match crate::grpc_impl::common::CommonConverters::parse_instance_id(&instance_id) {
                Ok(originator) => originator,
                Err(err) => return Err(anyhow::anyhow!("invalid originator field: {}", err)),
            },
            None => return Err(anyhow::anyhow!("missing originator field")),
        },
        serialized_data: api_instance.serialized_state.clone(),
    })
}

fn serialize_output_event_data(event: &crate::guest_api_host::OutputEventData) -> crate::grpc_impl::api::OutputEventData {
    crate::grpc_impl::api::OutputEventData {
        originator: Some(crate::grpc_impl::common::CommonConverters::serialize_instance_id(&event.originator)),
        alias: event.alias.clone(),
        msg: event.msg.clone(),
    }
}

fn serialize_output_event_data_raw(event: &crate::guest_api_host::OutputEventDataRaw) -> crate::grpc_impl::api::OutputEventDataRaw {
    crate::grpc_impl::api::OutputEventDataRaw {
        originator: Some(crate::grpc_impl::common::CommonConverters::serialize_instance_id(&event.originator)),
        dst: Some(crate::grpc_impl::common::CommonConverters::serialize_instance_id(&event.dst)),
        msg: event.msg.clone(),
    }
}

fn serialize_telemetry_log_event(event: &crate::guest_api_host::TelemetryLogEvent) -> crate::grpc_impl::api::TelemetryLogEvent {
    crate::grpc_impl::api::TelemetryLogEvent {
        originator: Some(crate::grpc_impl::common::CommonConverters::serialize_instance_id(&event.originator)),
        log_level: match event.log_level {
            crate::guest_api_host::TelemetryLogLevel::Error => crate::grpc_impl::api::TelemetryLogLevel::LogError as i32,
            crate::guest_api_host::TelemetryLogLevel::Warn => crate::grpc_impl::api::TelemetryLogLevel::LogWarn as i32,
            crate::guest_api_host::TelemetryLogLevel::Info => crate::grpc_impl::api::TelemetryLogLevel::LogInfo as i32,
            crate::guest_api_host::TelemetryLogLevel::Debug => crate::grpc_impl::api::TelemetryLogLevel::LogDebug as i32,
            crate::guest_api_host::TelemetryLogLevel::Trace => crate::grpc_impl::api::TelemetryLogLevel::LogTrace as i32,
        },
        msg: event.msg.clone(),
        target: event.target.clone(),
    }
}

fn serialize_delayed_event_data(event: &crate::guest_api_host::DelayedEventData) -> crate::grpc_impl::api::DelayedEventData {
    crate::grpc_impl::api::DelayedEventData {
        originator: Some(crate::grpc_impl::common::CommonConverters::serialize_instance_id(&event.originator)),
        delay: event.delay,
        alias: event.alias.clone(),
        msg: event.msg.clone(),
    }
}

fn serialize_sync_data(event: &crate::guest_api_host::SyncData) -> crate::grpc_impl::api::SyncData {
    crate::grpc_impl::api::SyncData {
        originator: Some(crate::grpc_impl::common::CommonConverters::serialize_instance_id(&event.originator)),
        serialized_state: event.serialized_data.clone(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::guest_api_host::DelayedEventData;
    use crate::guest_api_host::OutputEventData;
    use crate::guest_api_host::OutputEventDataRaw;
    use crate::guest_api_host::SyncData;
    use crate::guest_api_host::TelemetryLogEvent;
    use crate::guest_api_host::TelemetryLogLevel;
    use edgeless_api_core::instance_id::InstanceId;

    #[test]
    fn serialize_deserialize_output_event_data() {
        let messages = vec![
            OutputEventData {
                originator: edgeless_api_core::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
                alias: "".to_string(),
                msg: vec![],
            },
            OutputEventData {
                originator: edgeless_api_core::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
                alias: "my-fun".to_string(),
                msg: vec![0, 42, 0, 42, 99],
            },
        ];
        for msg in messages {
            match parse_output_event_data(&serialize_output_event_data(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_output_event_data_raw() {
        let messages = vec![
            OutputEventDataRaw {
                originator: edgeless_api_core::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
                dst: InstanceId::none(),
                msg: vec![],
            },
            OutputEventDataRaw {
                originator: edgeless_api_core::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
                dst: InstanceId::new(uuid::Uuid::new_v4()),
                msg: vec![0, 42, 0, 42, 99],
            },
        ];
        for msg in messages {
            match parse_output_event_data_raw(&serialize_output_event_data_raw(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_telemetry_log_event() {
        let mut messages = vec![];
        for log_level in vec![
            TelemetryLogLevel::Error,
            TelemetryLogLevel::Warn,
            TelemetryLogLevel::Info,
            TelemetryLogLevel::Debug,
            TelemetryLogLevel::Trace,
        ] {
            messages.push(TelemetryLogEvent {
                originator: edgeless_api_core::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
                log_level: log_level.clone(),
                msg: "".to_string(),
                target: "".to_string(),
            });
            messages.push(TelemetryLogEvent {
                originator: edgeless_api_core::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
                log_level,
                msg: "my-event".to_string(),
                target: "my-target".to_string(),
            });
        }
        for msg in messages {
            match parse_telemetry_log_event(&serialize_telemetry_log_event(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_delayed_event_data() {
        let messages = vec![
            DelayedEventData {
                originator: edgeless_api_core::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
                delay: 0_u64,
                alias: "".to_string(),
                msg: vec![],
            },
            DelayedEventData {
                originator: edgeless_api_core::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
                delay: 42_u64,
                alias: "my-fun".to_string(),
                msg: vec![0, 42, 0, 42, 99],
            },
        ];
        for msg in messages {
            match parse_delayed_event_data(&serialize_delayed_event_data(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_sync_data() {
        let messages = vec![
            SyncData {
                originator: edgeless_api_core::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
                serialized_data: vec![],
            },
            SyncData {
                originator: edgeless_api_core::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
                serialized_data: vec![0, 42, 0, 42, 99],
            },
        ];
        for msg in messages {
            match parse_sync_data(&serialize_sync_data(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }
}
