// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::str::FromStr;

pub struct Metrics {
    pub function_execution_times: std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<f64>>,
}

/// Non thread-safe data structure holding performance-related per-node metrics.
pub struct PerformanceTarget {
    metrics: Metrics,
}

impl Default for PerformanceTarget {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceTarget {
    pub fn new() -> Self {
        Self {
            metrics: Metrics {
                function_execution_times: std::collections::HashMap::new(),
            },
        }
    }

    /// Return the current metrics and reset them.
    pub fn get_metrics(&mut self) -> Metrics {
        Metrics {
            function_execution_times: std::mem::take(&mut self.metrics.function_execution_times),
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
                        let res = self.metrics.function_execution_times.entry(function_id).or_default();
                        res.push(lat.as_secs_f64());
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
            target: std::sync::Arc::new(std::sync::Mutex::new(PerformanceTarget::new())),
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
        let mut target = PerformanceTarget::new();
        let fid = uuid::Uuid::new_v4();
        let event_tags = std::collections::BTreeMap::from([("FUNCTION_ID".to_string(), fid.to_string())]);

        assert!(target.get_metrics().function_execution_times.is_empty());

        let mut expected = vec![];
        for i in 0..10 {
            expected.push(i as f64);
            target.handle(
                &crate::telemetry_events::TelemetryEvent::FunctionInit(std::time::Duration::from_secs(999)),
                &event_tags,
            );
            target.handle(
                (&crate::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(std::time::Duration::from_secs(i))),
                &event_tags,
            );
        }
        let metrics = target.get_metrics();
        assert_eq!(Some(expected), metrics.function_execution_times.get(&fid).cloned());

        assert!(target.get_metrics().function_execution_times.is_empty());
    }
}
