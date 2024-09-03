// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::str::FromStr;

pub struct Metrics {
    function_execution_times: std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<f32>>,
}

/// Data structure holding performance-related per-node metrics.
pub struct PerformanceTarget {
    metrics: std::sync::Arc<std::sync::Mutex<Metrics>>,
}

impl PerformanceTarget {
    pub fn new() -> Self {
        Self {
            metrics: std::sync::Arc::new(std::sync::Mutex::new(Metrics {
                function_execution_times: std::collections::HashMap::new(),
            })),
        }
    }

    /// Return the current metrics and reset them.
    pub fn get_metrics(&mut self) -> Metrics {
        let mut metrics = self.metrics.lock().expect("Could not lock mutex");
        Metrics {
            function_execution_times: std::mem::take(&mut metrics.function_execution_times),
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
                        let mut metrics = self.metrics.lock().expect("Could not lock mutex");
                        let res = metrics.function_execution_times.entry(function_id).or_insert(vec![]);
                        res.push(lat.as_secs_f32());
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
            expected.push(i as f32);
            target.handle(
                &crate::telemetry_events::TelemetryEvent::FunctionInit(std::time::Duration::from_secs(999)),
                &event_tags,
            );
            target.handle(
                &&crate::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(std::time::Duration::from_secs(i)),
                &event_tags,
            );
        }
        let metrics = target.get_metrics();
        assert_eq!(Some(expected), metrics.function_execution_times.get(&fid).cloned());

        assert!(target.get_metrics().function_execution_times.is_empty());
    }
}
