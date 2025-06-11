// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::str::FromStr;

#[derive(Default)]
pub struct Metrics {
    pub function_execution_times:
        std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::node_registration::Sample>>,
    pub function_transfer_times:
        std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::node_registration::Sample>>,
    pub function_log_entries:
        std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::node_registration::FunctionLogEntry>>,
}

/// Non thread-safe data structure holding performance-related per-node metrics.
#[derive(Default)]
pub struct PerformanceTarget {
    metrics: Metrics,
}

impl PerformanceTarget {
    /// Return the current metrics and reset them.
    pub fn get_metrics(&mut self) -> Metrics {
        Metrics {
            function_execution_times: std::mem::take(&mut self.metrics.function_execution_times),
            function_transfer_times: std::mem::take(&mut self.metrics.function_transfer_times),
            function_log_entries: std::mem::take(&mut self.metrics.function_log_entries),
        }
    }
}

impl crate::telemetry_events::EventProcessor for PerformanceTarget {
    fn handle(
        &mut self,
        event: &crate::telemetry_events::TelemetryEvent,
        event_tags: &std::collections::BTreeMap<String, String>,
    ) -> crate::telemetry_events::TelemetryProcessingResult {
        match event {
            crate::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(lat) => {
                if let Some(function_id) = event_tags.get("FUNCTION_ID") {
                    if let Ok(function_id) = uuid::Uuid::from_str(function_id) {
                        let now = chrono::Utc::now();
                        let res = self.metrics.function_execution_times.entry(function_id).or_default();
                        res.push(edgeless_api::node_registration::Sample {
                            timestamp_sec: now.timestamp(),
                            timestamp_ns: now.timestamp_subsec_nanos(),
                            sample: lat.as_secs_f64(),
                        });
                    }
                }
            }
            crate::telemetry_events::TelemetryEvent::FunctionTransfer(lat) => {
                if let Some(function_id) = event_tags.get("FUNCTION_ID") {
                    if let Ok(function_id) = uuid::Uuid::from_str(function_id) {
                        let now = chrono::Utc::now();
                        let res = self.metrics.function_transfer_times.entry(function_id).or_default();
                        res.push(edgeless_api::node_registration::Sample {
                            timestamp_sec: now.timestamp(),
                            timestamp_ns: now.timestamp_subsec_nanos(),
                            sample: lat.as_secs_f64(),
                        });
                    }
                }
            }
            crate::telemetry_events::TelemetryEvent::FunctionLogEntry(_lvl, target, message) => {
                if let Some(function_id) = event_tags.get("FUNCTION_ID") {
                    if let Ok(function_id) = uuid::Uuid::from_str(function_id) {
                        let now = chrono::Utc::now();
                        let res = self.metrics.function_log_entries.entry(function_id).or_default();
                        res.push(edgeless_api::node_registration::FunctionLogEntry {
                            timestamp_sec: now.timestamp(),
                            timestamp_ns: now.timestamp_subsec_nanos(),
                            target: target.to_string(),
                            message: message.to_string(),
                        });
                    }
                }
            }
            _ => {
                return crate::telemetry_events::TelemetryProcessingResult::PASSED;
            }
        }
        crate::telemetry_events::TelemetryProcessingResult::PROCESSED
    }
}

/// Thread-safe wrapper of `PerformanceTarget`.
#[derive(Clone)]
pub struct PerformanceTargetInner {
    target: std::sync::Arc<std::sync::Mutex<PerformanceTarget>>,
}

impl Default for PerformanceTargetInner {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceTargetInner {
    /// Create a new empty `PerformanceTarget`.
    pub fn new() -> Self {
        Self {
            target: std::sync::Arc::new(std::sync::Mutex::new(PerformanceTarget::default())),
        }
    }

    /// Return the current metrics and reset them.
    pub fn get_metrics(&mut self) -> Metrics {
        self.target.lock().expect("Could not lock mutex").get_metrics()
    }
}

/// Wrapper of `PerformanceTargetInner` that implemented the `EventProcessor`
/// interface.
pub struct PerformanceTargetOuter {
    inner: PerformanceTargetInner,
}

impl PerformanceTargetOuter {
    pub fn new(inner: PerformanceTargetInner) -> Self {
        Self { inner }
    }
}

impl crate::telemetry_events::EventProcessor for PerformanceTargetOuter {
    fn handle(
        &mut self,
        event: &crate::telemetry_events::TelemetryEvent,
        event_tags: &std::collections::BTreeMap<String, String>,
    ) -> crate::telemetry_events::TelemetryProcessingResult {
        self.inner.target.lock().expect("Could not lock mutex").handle(event, event_tags)
    }
}

#[cfg(test)]
mod tests {
    use crate::telemetry_events::EventProcessor;

    use super::*;

    #[test]
    fn test_performance_target_get_metrics() {
        let mut target = PerformanceTarget::default();
        let fid = uuid::Uuid::new_v4();
        let event_tags = std::collections::BTreeMap::from([("FUNCTION_ID".to_string(), fid.to_string())]);

        let metrics = target.get_metrics();
        assert!(metrics.function_execution_times.is_empty());
        assert!(metrics.function_transfer_times.is_empty());
        assert!(metrics.function_log_entries.is_empty());

        let mut expected_execution = vec![];
        let mut expected_transfer = vec![];
        let mut expected_log_entries = vec![];
        for i in 0..10 {
            expected_execution.push(i as f64);
            expected_transfer.push((1000 + i) as f64);
            expected_log_entries.push((format!("target{}", i), format!("message{}", i)));
            target.handle(
                &crate::telemetry_events::TelemetryEvent::FunctionInit(std::time::Duration::from_secs(999)),
                &event_tags,
            );
            target.handle(
                &crate::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(std::time::Duration::from_secs(
                    *expected_execution.last().unwrap() as u64,
                )),
                &event_tags,
            );
            target.handle(
                &crate::telemetry_events::TelemetryEvent::FunctionTransfer(std::time::Duration::from_secs(*expected_transfer.last().unwrap() as u64)),
                &event_tags,
            );
            let log_target_message = expected_log_entries.last().cloned().unwrap();
            target.handle(
                &crate::telemetry_events::TelemetryEvent::FunctionLogEntry(
                    crate::telemetry_events::TelemetryLogLevel::Trace,
                    log_target_message.0,
                    log_target_message.1,
                ),
                &event_tags,
            );
        }

        let metrics = target.get_metrics();

        let samples = metrics.function_execution_times.get(&fid).cloned().unwrap();
        assert_eq!(expected_execution, samples.iter().map(|x| x.sample).collect::<Vec<f64>>());

        let samples = metrics.function_transfer_times.get(&fid).cloned().unwrap();
        assert_eq!(expected_transfer, samples.iter().map(|x| x.sample).collect::<Vec<f64>>());

        let log_entries = metrics.function_log_entries.get(&fid).cloned().unwrap();
        assert_eq!(
            expected_log_entries,
            log_entries
                .iter()
                .map(|x| (x.target.clone(), x.message.clone()))
                .collect::<Vec<(String, String)>>()
        );

        let metrics = target.get_metrics();
        assert!(metrics.function_execution_times.is_empty());
        assert!(metrics.function_transfer_times.is_empty());
        assert!(metrics.function_log_entries.is_empty());
    }
}
