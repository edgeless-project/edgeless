// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

extern crate redis;
use std::io::Write;

pub struct CsvDumper {
    additional_fields: String,
    outfile: Option<std::fs::File>,
}

impl CsvDumper {
    pub fn new(additional_fields: String, additional_header: String, filename: &str, append: bool) -> anyhow::Result<Self> {
        let mut additional_fields = additional_fields;
        let mut outfile = None;
        if !filename.is_empty() {
            let header = !append
                || match std::fs::metadata(filename) {
                    Ok(metadata) => metadata.len() == 0,
                    Err(_) => true,
                };

            outfile = Some(
                std::fs::OpenOptions::new()
                    .write(true)
                    .append(append)
                    .create(true)
                    .truncate(!append)
                    .open(filename)?,
            );

            let mut additional_header = additional_header;
            anyhow::ensure!(
                additional_header.split(",").count() == additional_fields.split(",").count(),
                "different number of comma-separated values in additional headers vs. fields"
            );

            if !additional_header.is_empty() {
                anyhow::ensure!(!additional_fields.is_empty(), "empty additional fields with non-empty additional header");
                additional_header += ",";
                additional_fields += ",";
            }
            if header {
                if let Some(outfile) = &mut outfile {
                    writeln!(outfile, "{}timestamp,metric,target,value", additional_header)?;
                }
            }
        }

        Ok(Self { additional_fields, outfile })
    }

    pub fn add(&mut self, metric: &str, target: &str, value: &str) {
        if let Some(outfile) = &mut self.outfile {
            let _ = writeln!(
                outfile,
                "{}{},{},{},{}",
                self.additional_fields,
                crate::utils::timestamp_now(),
                metric,
                target,
                value
            );
        }
    }
}
