// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    #[cfg(not(feature = "std"))]
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            match record.args().as_str() {
                Some(data) => {
                    telemetry_log(rust_to_api(record.level()) as usize, record.target(), data)
                }
                _ => {
                    telemetry_log(
                        rust_to_api(record.level()) as usize,
                        record.target(),
                        "Unsupported Message Arguments",
                    );
                }
            }
        }
    }
    #[cfg(feature = "std")]
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            super::telemetry_log(
                rust_to_api(record.level()) as usize,
                record.target(),
                &record.args().to_string(),
            );
        }
    }

    fn flush(&self) {}
}

static LOGGER: Logger = Logger;

pub fn init_logger() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Debug))
        .unwrap();
}

pub fn rust_to_api(lvl: log::Level) -> u32 {
    match lvl {
        log::Level::Error => 1,
        log::Level::Warn => 2,
        log::Level::Info => 3,
        log::Level::Debug => 4,
        log::Level::Trace => 5,
    }
}
