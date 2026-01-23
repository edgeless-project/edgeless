// SPDX-FileCopyrightText: © 2025 Siemens AG <zalewski.lukasz@siemens.com>
// SPDX-License-Identifier: MIT

use std::io::Write;

#[derive(Debug, Clone)]
pub enum TraceEvent {
    SpanStart {
        correlation_id: uuid::Uuid,
        parent_id: Option<uuid::Uuid>,
        name: String,
    },
    SpanEnd {
        correlation_id: uuid::Uuid,
    },
    Log {
        correlation_id: Option<uuid::Uuid>,
        level: String,
        message: String,
    },
}

pub struct ControlPlaneTracer {
    target: std::sync::Arc<std::sync::Mutex<CsvTracerTarget>>,
}

impl Clone for ControlPlaneTracer {
    fn clone(&self) -> Self {
        Self { target: self.target.clone() }
    }
}

impl ControlPlaneTracer {
    pub fn new(output_path: String) -> anyhow::Result<Self> {
        Ok(Self {
            target: std::sync::Arc::new(std::sync::Mutex::new(CsvTracerTarget::new(output_path)?)),
        })
    }

    pub fn start_span(&self, name: &str) -> TraceSpan {
        let correlation_id = uuid::Uuid::new_v4();
        let _ = self.target.lock().unwrap().record(TraceEvent::SpanStart {
            correlation_id,
            parent_id: None,
            name: name.to_string(),
        });
        TraceSpan {
            correlation_id,
            target: self.target.clone(),
            ended: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn log(&self, level: &str, message: &str) {
        let _ = self.target.lock().unwrap().record(TraceEvent::Log {
            correlation_id: None,
            level: level.to_string(),
            message: message.to_string(),
        });
    }
}

pub struct TraceSpan {
    correlation_id: uuid::Uuid,
    target: std::sync::Arc<std::sync::Mutex<CsvTracerTarget>>,
    ended: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl TraceSpan {
    pub fn child(&self, name: &str) -> TraceSpan {
        let child_id = uuid::Uuid::new_v4();
        let _ = self.target.lock().unwrap().record(TraceEvent::SpanStart {
            correlation_id: child_id,
            parent_id: Some(self.correlation_id),
            name: name.to_string(),
        });
        TraceSpan {
            correlation_id: child_id,
            target: self.target.clone(),
            ended: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn log(&self, level: &str, message: &str) {
        let _ = self.target.lock().unwrap().record(TraceEvent::Log {
            correlation_id: Some(self.correlation_id),
            level: level.to_string(),
            message: message.to_string(),
        });
    }

    pub fn id(&self) -> uuid::Uuid {
        self.correlation_id
    }

    /// Manually end the span. Safe to call multiple times - only the first call records the end.
    pub fn end(&self) {
        if !self.ended.swap(true, std::sync::atomic::Ordering::SeqCst) {
            let _ = self.target.lock().unwrap().record(TraceEvent::SpanEnd {
                correlation_id: self.correlation_id,
            });
        }
    }
}

// automatically record span end on drop of the object
impl Drop for TraceSpan {
    fn drop(&mut self) {
        if !self.ended.swap(true, std::sync::atomic::Ordering::SeqCst) {
            let _ = self.target.lock().unwrap().record(TraceEvent::SpanEnd {
                correlation_id: self.correlation_id,
            });
        }
    }
}

struct CsvTracerTarget {
    sender: std::sync::mpsc::Sender<String>,
}

impl CsvTracerTarget {
    fn new(output_path: String) -> anyhow::Result<Self> {
        let (sender, receiver) = std::sync::mpsc::channel::<String>();

        // Spawn background thread for disk writes
        std::thread::spawn(move || {
            let writer: Box<dyn Write + Send> = if output_path.is_empty() || output_path == "-" {
                Box::new(std::io::stdout())
            } else {
                match std::fs::File::create(&output_path) {
                    Ok(file) => Box::new(file),
                    Err(e) => {
                        eprintln!("Failed to create trace file {}: {}", output_path, e);
                        return;
                    }
                }
            };

            let mut writer = writer;

            // Write header
            if let Err(e) = writeln!(
                writer,
                "timestamp_sec,timestamp_ns,event_type,correlation_id,parent_id,name,level,message"
            ) {
                eprintln!("Failed to write trace header: {}", e);
                return;
            }

            // Process messages from the channel
            while let Ok(line) = receiver.recv() {
                if let Err(e) = writeln!(writer, "{}", line) {
                    eprintln!("Failed to write trace line: {}", e);
                    break;
                }
                if let Err(e) = writer.flush() {
                    eprintln!("Failed to flush trace writer: {}", e);
                    break;
                }
            }
        });

        Ok(Self { sender })
    }

    fn record(&self, event: TraceEvent) -> anyhow::Result<()> {
        let now = chrono::Utc::now();
        let timestamp_sec = now.timestamp();
        let timestamp_ns = now.timestamp_subsec_nanos();

        let line = match event {
            TraceEvent::SpanStart {
                correlation_id,
                parent_id,
                name,
            } => format!(
                "{},{},span_start,{},{},{},,",
                timestamp_sec,
                timestamp_ns,
                correlation_id,
                parent_id.map(|id| id.to_string()).unwrap_or_default(),
                name
            ),
            TraceEvent::SpanEnd { correlation_id } => format!("{},{},span_end,{},,,,", timestamp_sec, timestamp_ns, correlation_id),
            TraceEvent::Log {
                correlation_id,
                level,
                message,
            } => format!(
                "{},{},log,{},,,,{},\"{}\"",
                timestamp_sec,
                timestamp_ns,
                correlation_id.map(|id| id.to_string()).unwrap_or_default(),
                level,
                message.replace("\"", "\"\"")
            ),
        };

        // Send to background thread - this is non-blocking
        self.sender.send(line).map_err(|e| anyhow::anyhow!("Failed to send trace event: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_plane_api() {
        let handle = ControlPlaneTracer::new(String::new()).unwrap();

        let span = handle.start_span("request");
        span.log("info", "processing request");

        {
            let child = span.child("database_query");
            child.log("debug", "executing query");
        }

        {
            let child = span.child("cache_lookup");
            child.log("debug", "checking cache");
        }

        span.log("info", "request completed");

        handle.log("warn", "global log message");
    }

    #[test]
    fn test_csv_events() {
        let target = CsvTracerTarget::new(String::new()).unwrap();

        let correlation_id = uuid::Uuid::new_v4();
        let child_id = uuid::Uuid::new_v4();

        target
            .record(TraceEvent::SpanStart {
                correlation_id,
                parent_id: None,
                name: "test_span".to_string(),
            })
            .unwrap();

        target
            .record(TraceEvent::SpanStart {
                correlation_id: child_id,
                parent_id: Some(correlation_id),
                name: "child_span".to_string(),
            })
            .unwrap();

        target
            .record(TraceEvent::Log {
                correlation_id: Some(child_id),
                level: "info".to_string(),
                message: "test message".to_string(),
            })
            .unwrap();

        target.record(TraceEvent::SpanEnd { correlation_id: child_id }).unwrap();

        target
            .record(TraceEvent::SpanEnd {
                correlation_id: correlation_id,
            })
            .unwrap();

        target
            .record(TraceEvent::Log {
                correlation_id: None,
                level: "warn".to_string(),
                message: "uncorrelated log".to_string(),
            })
            .unwrap();
    }
}
