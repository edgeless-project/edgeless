// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Debug, PartialEq, Eq)]
pub enum TelemetryLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

pub fn api_to_telemetry(lvl: String) -> TelemetryLogLevel {
    match lvl.as_str() {
        "Trace" => TelemetryLogLevel::Trace,
        "Debug" => TelemetryLogLevel::Debug,
        "Info" => TelemetryLogLevel::Info,
        "Warn" => TelemetryLogLevel::Warn,
        _ => TelemetryLogLevel::Error,
    }
}

pub fn telemetry_to_api(lvl: TelemetryLogLevel) -> String {
    match lvl {
        TelemetryLogLevel::Trace => "Trace".to_string(),
        TelemetryLogLevel::Debug => "Debug".to_string(),
        TelemetryLogLevel::Info => "Info".to_string(),
        TelemetryLogLevel::Warn => "Warn".to_string(),
        TelemetryLogLevel::Error => "Error".to_string(),
    }
}

fn to_log_level(lvl: &TelemetryLogLevel) -> log::Level {
    match lvl {
        TelemetryLogLevel::Error => log::Level::Error,
        TelemetryLogLevel::Warn => log::Level::Warn,
        TelemetryLogLevel::Info => log::Level::Info,
        TelemetryLogLevel::Debug => log::Level::Debug,
        TelemetryLogLevel::Trace => log::Level::Trace,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FunctionExitStatus {
    Ok,
    InternalError,
    CodeError,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TelemetryEvent {
    FunctionInstantiate(std::time::Duration),
    FunctionInit(std::time::Duration),
    FunctionLogEntry(TelemetryLogLevel, String, String), // (_, target, msg)
    FunctionInvocationCompleted(std::time::Duration),
    FunctionStop(std::time::Duration),
    FunctionExit(FunctionExitStatus),
    FunctionTransfer(std::time::Duration),
}

#[derive(Clone)]
pub struct TelemetryHandle {
    handle_tags: std::collections::BTreeMap<String, String>,
    sender: tokio::sync::mpsc::UnboundedSender<TelemetryProcessorInput>,
}

pub trait TelemetryHandleAPI: Send {
    fn observe(&mut self, event: TelemetryEvent, event_tags: std::collections::BTreeMap<String, String>);
    fn fork(&mut self, child_tags: std::collections::BTreeMap<String, String>) -> Box<dyn TelemetryHandleAPI>;
}

impl TelemetryHandleAPI for TelemetryHandle {
    fn observe(&mut self, event: TelemetryEvent, event_tags: std::collections::BTreeMap<String, String>) {
        let mut event_tags = event_tags;
        let mut merged_tags = self.handle_tags.clone();
        merged_tags.append(&mut event_tags);

        self.sender.send(TelemetryProcessorInput::TelemetryEvent(event, merged_tags)).unwrap();
    }

    fn fork(&mut self, child_tags: std::collections::BTreeMap<String, String>) -> Box<dyn TelemetryHandleAPI> {
        let mut child_tags = child_tags;
        let mut merged_tags = self.handle_tags.clone();
        merged_tags.append(&mut child_tags);
        Box::new(TelemetryHandle {
            handle_tags: merged_tags,
            sender: self.sender.clone(),
        })
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum TelemetryProcessingResult {
    PASSED,
    PROCESSED,
    FINAL,
}

#[derive(Debug)]
enum TelemetryProcessorInput {
    TelemetryEvent(TelemetryEvent, std::collections::BTreeMap<String, String>),
}

pub trait EventProcessor: Sync + Send {
    fn handle(&mut self, event: &TelemetryEvent, event_tags: &std::collections::BTreeMap<String, String>) -> TelemetryProcessingResult;
}

#[derive(Default)]
struct EventLogger {}

impl EventProcessor for EventLogger {
    fn handle(&mut self, event: &TelemetryEvent, event_tags: &std::collections::BTreeMap<String, String>) -> TelemetryProcessingResult {
        if let TelemetryEvent::FunctionLogEntry(_, _, msg) = event {
            if let Some(function_id) = event_tags.get("FUNCTION_ID") {
                if let Some(node_id) = event_tags.get("NODE_ID") {
                    log::info!("Function log, node_id=({:?}), function_id=({:?}): {:?}", node_id, function_id, msg)
                }
            }
        }
        TelemetryProcessingResult::PROCESSED
    }
}

struct TelemetryProcessorInner {
    processing_chain: Vec<Box<dyn EventProcessor>>,
    receiver: tokio::sync::mpsc::UnboundedReceiver<TelemetryProcessorInput>,
}

impl TelemetryProcessorInner {
    async fn run(&mut self) {
        while let Some(val) = self.receiver.recv().await {
            match val {
                TelemetryProcessorInput::TelemetryEvent(event, event_tags) => {
                    self.handle(event, event_tags).await;
                }
            }
        }
    }

    async fn handle(&mut self, event: TelemetryEvent, event_tags: std::collections::BTreeMap<String, String>) {
        for processor in &mut self.processing_chain {
            let processing_result = processor.handle(&event, &event_tags);
            if processing_result == TelemetryProcessingResult::FINAL {
                break;
            }
        }
    }
}

pub struct TelemetryProcessor {
    sender: tokio::sync::mpsc::UnboundedSender<TelemetryProcessorInput>,
}

impl TelemetryProcessor {
    ///
    /// Create a sub-system to process telemetry events.
    ///
    /// Parameters:
    /// - `prometheus_url`: HTTP end-point to which to bind a web server
    ///   providing an interface suitable to be scraped by Prometheus
    ///   (https://prometheus.io/); if empty then the server is not started
    /// - `performance_target`: optional target that collects samples about
    ///   performance-related events
    ///
    pub async fn new(prometheus_url: String, performance_target: Option<crate::performance_target::PerformanceTargetInner>) -> anyhow::Result<Self> {
        let mut processing_chain: Vec<Box<dyn EventProcessor>> = vec![];

        // Add the performance target, if present.
        if let Some(performance_target) = performance_target {
            processing_chain.push(Box::new(crate::performance_target::PerformanceTargetOuter::new(performance_target)));
        }

        // Create and add the Prometheus target, if required.
        if !prometheus_url.is_empty() {
            match edgeless_api::util::parse_http_host(&prometheus_url) {
                Ok((_, ip, port)) => {
                    processing_chain.push(Box::new(
                        crate::prometheus_target::PrometheusEventTarget::new(&format!("{}:{}", &ip, port)).await,
                    ));
                }
                Err(err) => anyhow::bail!("could not create Prometheus server: {}", err),
            }
        }

        // Created and the log target, if required.
        processing_chain.push(Box::new(EventLogger::default()));

        // Create a channel to receive telemetry events and the processor that
        // will handle them, spawned in a dedicated task.
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<TelemetryProcessorInput>();
        let inner = TelemetryProcessorInner { processing_chain, receiver };
        tokio::spawn(async move {
            let mut inner = inner;
            inner.run().await;
        });

        Ok(Self { sender })
    }

    pub fn get_handle(&self, handle_tags: std::collections::BTreeMap<String, String>) -> TelemetryHandle {
        TelemetryHandle {
            handle_tags,
            sender: self.sender.clone(),
        }
    }
}
