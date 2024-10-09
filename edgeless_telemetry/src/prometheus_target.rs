// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use warp::Filter;

/// Prometheus collects metrics from targets by scraping metrics HTTP targets. This struct defines that.
pub struct PrometheusEventTarget {
    _registry: std::sync::Arc<tokio::sync::Mutex<prometheus_client::registry::Registry>>,
    function_count: prometheus_client::metrics::family::Family<RuntimeLabels, prometheus_client::metrics::gauge::Gauge>,
    execution_times: prometheus_client::metrics::family::Family<ExecutionLabels, prometheus_client::metrics::histogram::Histogram>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
struct RuntimeLabels {
    node_id: String,
    function_type: String,
}

// TODO: add additional labels like class_spec, function_name
#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
struct FunctionLabels {
    node_id: String,
    function_id: String,
    function_type: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelValue)]
enum InvocationType {
    Cast,
    Call,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
struct ExecutionLabels {
    node_id: String,
    function_id: String,
    function_type: String,
    invocation_type: InvocationType,
}

impl PrometheusEventTarget {
    pub async fn new(endpoint: &str) -> Self {
        let registry = std::sync::Arc::new(tokio::sync::Mutex::new(<prometheus_client::registry::Registry>::default()));

        let function_count = prometheus_client::metrics::family::Family::<RuntimeLabels, prometheus_client::metrics::gauge::Gauge>::default();

        let execution_times =
            prometheus_client::metrics::family::Family::<ExecutionLabels, prometheus_client::metrics::histogram::Histogram>::new_with_constructor(
                || {
                    let buckets = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
                    prometheus_client::metrics::histogram::Histogram::new(buckets.into_iter())
                },
            );

        registry.lock().await.register("function_count", "", function_count.clone());
        registry.lock().await.register("execution_times", "", execution_times.clone());

        let reg_clone = registry.clone();
        let socket_addr: std::net::SocketAddr = endpoint.parse().unwrap_or_else(|_| panic!("invalid endpoint: {}", &endpoint));
        tokio::spawn(async move {
            let metric_handler = warp::path("metrics").then(move || {
                let cloned = reg_clone.clone();
                async move {
                    let mut buffer = String::new();
                    match prometheus_client::encoding::text::encode(&mut buffer, &*cloned.lock().await) {
                        Ok(_) => warp::http::Response::builder().body(buffer),
                        Err(_) => warp::http::Response::builder().status(500).body("".to_string()),
                    }
                }
            });

            warp::serve(metric_handler).run(socket_addr).await;
        });

        Self {
            _registry: registry,
            function_count,
            execution_times,
        }
    }
}

impl crate::telemetry_events::EventProcessor for PrometheusEventTarget {
    fn handle(
        &mut self,
        event: &crate::telemetry_events::TelemetryEvent,
        event_tags: &std::collections::BTreeMap<String, String>,
    ) -> crate::telemetry_events::TelemetryProcessingResult {
        match event {
            crate::telemetry_events::TelemetryEvent::FunctionInstantiate(_) => {
                if let (Some(node_id), Some(function_type)) = (event_tags.get("NODE_ID"), event_tags.get("FUNCTION_TYPE")) {
                    self.function_count
                        .get_or_create(&RuntimeLabels {
                            node_id: node_id.to_string(),
                            function_type: function_type.to_string(),
                        })
                        .inc();
                }
            }
            crate::telemetry_events::TelemetryEvent::FunctionExit(_) => {
                if let (Some(node_id), Some(function_type)) = (event_tags.get("NODE_ID"), event_tags.get("FUNCTION_TYPE")) {
                    self.function_count
                        .get_or_create(&RuntimeLabels {
                            node_id: node_id.to_string(),
                            function_type: function_type.to_string(),
                        })
                        .dec();
                }
            }
            crate::telemetry_events::TelemetryEvent::FunctionInvocationCompleted(lat) => {
                if let (Some(node_id), Some(function_id), Some(function_type), Some(invoction_type)) = (
                    event_tags.get("NODE_ID"),
                    event_tags.get("FUNCTION_ID"),
                    event_tags.get("FUNCTION_TYPE"),
                    event_tags.get("EVENT_TYPE"),
                ) {
                    self.execution_times
                        .get_or_create(&ExecutionLabels {
                            node_id: node_id.to_string(),
                            function_type: function_type.to_string(),
                            function_id: function_id.to_string(),
                            invocation_type: match invoction_type.as_str() {
                                "CALL" => InvocationType::Call,
                                _ => InvocationType::Cast,
                            },
                        })
                        .observe(lat.as_secs_f64())
                }
            }
            _ => {
                return crate::telemetry_events::TelemetryProcessingResult::PASSED;
            }
        }
        crate::telemetry_events::TelemetryProcessingResult::FINAL
    }
}
