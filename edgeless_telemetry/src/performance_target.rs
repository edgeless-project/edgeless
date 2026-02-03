// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::str::FromStr;

pub enum FunctionTime {
    Instantiate = 0,
    Init,
    Execution,
    Stop,
    Transfer,
}

pub type FunctionTimes = std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::node_registration::Sample>>;

#[derive(Default)]
pub struct Metrics {
    pub function_times: [FunctionTimes; 5],
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
            function_times: std::mem::take(&mut self.metrics.function_times),
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
        let new_sample = |lat: &std::time::Duration| {
            let now = chrono::Utc::now();
            edgeless_api::node_registration::Sample {
                timestamp_sec: now.timestamp(),
                timestamp_ns: now.timestamp_subsec_nanos(),
                sample: lat.as_secs_f64(),
            }
        };

        match event {
            crate::telemetry_events::TelemetryEvent::FunctionInstantiate(lat) => {
                if let Some(function_id) = event_tags.get("FUNCTION_ID")
                    && let Ok(function_id) = uuid::Uuid::from_str(function_id)
                {
                    let res = self.metrics.function_times[FunctionTime::Instantiate as usize]
                        .entry(function_id)
                        .or_default();
                    res.push(new_sample(lat));
                }
            }
            crate::telemetry_events::TelemetryEvent::FunctionInit(lat) => {
                if let Some(function_id) = event_tags.get("FUNCTION_ID")
                    && let Ok(function_id) = uuid::Uuid::from_str(function_id)
                {
                    let res = self.metrics.function_times[FunctionTime::Init as usize].entry(function_id).or_default();
                    res.push(new_sample(lat));
                }
            }
            crate::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(lat) => {
                if let Some(function_id) = event_tags.get("FUNCTION_ID")
                    && let Ok(function_id) = uuid::Uuid::from_str(function_id)
                {
                    let res = self.metrics.function_times[FunctionTime::Execution as usize]
                        .entry(function_id)
                        .or_default();
                    res.push(new_sample(lat));
                }
            }
            crate::telemetry_events::TelemetryEvent::FunctionStop(lat) => {
                if let Some(function_id) = event_tags.get("FUNCTION_ID")
                    && let Ok(function_id) = uuid::Uuid::from_str(function_id)
                {
                    let res = self.metrics.function_times[FunctionTime::Stop as usize].entry(function_id).or_default();
                    res.push(new_sample(lat));
                }
            }
            crate::telemetry_events::TelemetryEvent::FunctionTransfer(lat) => {
                if let Some(function_id) = event_tags.get("FUNCTION_ID")
                    && let Ok(function_id) = uuid::Uuid::from_str(function_id)
                {
                    let res = self.metrics.function_times[FunctionTime::Transfer as usize]
                        .entry(function_id)
                        .or_default();
                    res.push(new_sample(lat));
                }
            }
            crate::telemetry_events::TelemetryEvent::FunctionLogEntry(_lvl, target, message) => {
                if let Some(function_id) = event_tags.get("FUNCTION_ID")
                    && let Ok(function_id) = uuid::Uuid::from_str(function_id)
                {
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
        assert!(metrics.function_times[FunctionTime::Instantiate as usize].is_empty());
        assert!(metrics.function_times[FunctionTime::Init as usize].is_empty());
        assert!(metrics.function_times[FunctionTime::Execution as usize].is_empty());
        assert!(metrics.function_times[FunctionTime::Stop as usize].is_empty());
        assert!(metrics.function_times[FunctionTime::Transfer as usize].is_empty());
        assert!(metrics.function_log_entries.is_empty());

        let mut expected_instantiate = vec![];
        let mut expected_init = vec![];
        let mut expected_execution = vec![];
        let mut expected_stop = vec![];
        let mut expected_transfer = vec![];
        let mut expected_log_entries = vec![];
        for i in 0..10 {
            expected_instantiate.push(i as f64 * 2.0);
            expected_init.push(i as f64 * 3.0);
            expected_execution.push(i as f64 * 4.0);
            expected_stop.push(i as f64 * 5.0);
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

        let samples = metrics.function_times[FunctionTime::Instantiate as usize].get(&fid).cloned().unwrap();
        assert_eq!(expected_instantiate, samples.iter().map(|x| x.sample).collect::<Vec<f64>>());

        let samples = metrics.function_times[FunctionTime::Init as usize].get(&fid).cloned().unwrap();
        assert_eq!(expected_init, samples.iter().map(|x| x.sample).collect::<Vec<f64>>());

        let samples = metrics.function_times[FunctionTime::Execution as usize].get(&fid).cloned().unwrap();
        assert_eq!(expected_execution, samples.iter().map(|x| x.sample).collect::<Vec<f64>>());

        let samples = metrics.function_times[FunctionTime::Stop as usize].get(&fid).cloned().unwrap();
        assert_eq!(expected_stop, samples.iter().map(|x| x.sample).collect::<Vec<f64>>());

        let samples = metrics.function_times[FunctionTime::Transfer as usize].get(&fid).cloned().unwrap();
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
        assert!(metrics.function_times[FunctionTime::Instantiate as usize].is_empty());
        assert!(metrics.function_times[FunctionTime::Init as usize].is_empty());
        assert!(metrics.function_times[FunctionTime::Execution as usize].is_empty());
        assert!(metrics.function_times[FunctionTime::Stop as usize].is_empty());
        assert!(metrics.function_times[FunctionTime::Transfer as usize].is_empty());
        assert!(metrics.function_log_entries.is_empty());
    }
}
