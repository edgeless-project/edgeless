#[derive(Debug)]
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

#[derive(Debug)]
pub enum TelemetryEvent {
    FunctionInstantiate(std::time::Duration),
    FunctionInit(std::time::Duration),
    FunctionLogEntry(TelemetryLogLevel, String, String),
    FunctionInvocationCompleted(std::time::Duration),
    FunctionStop(std::time::Duration),
    FunctionExit,
}

#[derive(Clone)]
pub struct TelemetryHandle {
    handle_tags: std::collections::BTreeMap<String, String>,
    sender: tokio::sync::mpsc::UnboundedSender<TelemetryProcessorInput>,
}

impl TelemetryHandle {
    pub fn observe(&mut self, event: TelemetryEvent, event_tags: std::collections::BTreeMap<String, String>) {
        let mut event_tags = event_tags;
        let mut merged_tags = self.handle_tags.clone();
        merged_tags.append(&mut event_tags);

        self.sender.send(TelemetryProcessorInput::TelemetryEvent(event, merged_tags)).unwrap();
    }

    pub fn fork(&mut self, child_tags: std::collections::BTreeMap<String, String>) -> TelemetryHandle {
        let mut child_tags = child_tags;
        let mut merged_tags = self.handle_tags.clone();
        merged_tags.append(&mut child_tags);
        TelemetryHandle {
            handle_tags: merged_tags,
            sender: self.sender.clone(),
        }
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

struct EventLogger {}

impl EventProcessor for EventLogger {
    fn handle(&mut self, event: &TelemetryEvent, event_tags: &std::collections::BTreeMap<String, String>) -> TelemetryProcessingResult {
        println!("Event: {:?} , tags: {:?}", event, event_tags);
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
                    self.handle_event(event, event_tags).await;
                }
            }
        }
    }

    async fn handle_event(&mut self, event: TelemetryEvent, event_tags: std::collections::BTreeMap<String, String>) {
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
    pub async fn new(metrics_url: String) -> Self {
        let mut listen_port: u16 = 7003;
        if let Ok((_, _, port)) = edgeless_api::util::parse_http_host(&metrics_url) {
            listen_port = port;
        }

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<TelemetryProcessorInput>();

        let inner = TelemetryProcessorInner {
            processing_chain: vec![
                Box::new(crate::prometheus_target::PrometheusEventTarget::new(listen_port).await),
                Box::new(EventLogger {}),
            ],
            receiver: receiver,
        };

        tokio::spawn(async move {
            let mut inner = inner;
            inner.run().await;
        });

        Self { sender }
    }

    pub fn get_handle(&self, handle_tags: std::collections::BTreeMap<String, String>) -> TelemetryHandle {
        TelemetryHandle {
            handle_tags: handle_tags,
            sender: self.sender.clone(),
        }
    }
}
