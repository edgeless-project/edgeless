// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
pub mod dda;
pub mod file_log;
pub mod http_egress;
pub mod http_ingress;
#[cfg(feature = "rdkafka")]
pub mod kafka_egress;
pub mod metrics_collector;
pub mod ollama;
pub mod redis;
pub mod resource_provider_specs;
pub mod serverless;
pub mod sqlx;

fn observe_transfer(
    created: edgeless_api::function_instance::EventTimestamp,
    telemetry_handle: &mut Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
) -> chrono::DateTime<chrono::Utc> {
    let now = chrono::Utc::now();
    let created = chrono::DateTime::from_timestamp(created.secs, created.nsecs).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    let elapsed = (now - created).to_std().unwrap_or(std::time::Duration::ZERO);
    telemetry_handle.observe(
        edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionTransfer(elapsed),
        std::collections::BTreeMap::new(),
    );
    now
}

fn observe_execution(
    started: chrono::DateTime<chrono::Utc>,
    telemetry_handle: &mut Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    need_reply: bool,
) {
    let now = chrono::Utc::now();
    let elapsed = (now - started).to_std().unwrap_or(std::time::Duration::ZERO);
    let event_type = if need_reply { "CALL" } else { "CAST" };
    telemetry_handle.observe(
        edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(elapsed),
        std::collections::BTreeMap::from([("EVENT_TYPE".to_string(), event_type.to_string())]),
    );
}
