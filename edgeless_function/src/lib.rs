// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
pub mod api {
    wit_bindgen::generate!({world: "edgefunction", macro_export, export_macro_name: "export"});
}

pub fn rust_to_api(lvl: log::Level) -> String {
    match lvl {
        log::Level::Trace => "Trace".to_string(),
        log::Level::Debug => "Debug".to_string(),
        log::Level::Info => "Info".to_string(),
        log::Level::Warn => "Warn".to_string(),
        log::Level::Error => "Error".to_string(),
    }
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            api::telemetry_log(&rust_to_api(record.level()), record.target(), &record.args().to_string());
        }
    }

    fn flush(&self) {}
}

static LOGGER: Logger = Logger;

pub fn init_logger() {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Debug)).unwrap();
}
