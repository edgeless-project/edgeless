// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

extern crate redis;
use std::io::Write;

use redis::Commands;

pub struct RedisDumper {
    connection: redis::Connection,
    additional_fields: String,
    additional_header: String,
}

impl RedisDumper {
    pub fn new(redis_url: &str, additional_fields: String, additional_header: String) -> Result<Self, String> {
        match redis::Client::open(redis_url) {
            Ok(client) => match client.get_connection() {
                Ok(val) => Ok(Self {
                    connection: val,
                    additional_fields,
                    additional_header,
                }),
                Err(err) => Err(format!("could not open a Redis connection: {}", err)),
            },
            Err(err) => Err(format!("could not open a Redis connection: {}", err)),
        }
    }

    pub fn set(&mut self, k: &str, v: &str) {
        let _ = self.connection.set::<&str, &str, usize>(k, v);
    }

    fn open_file(filename: &str, append: bool, additional_header: &str) -> anyhow::Result<std::fs::File> {
        let header = !append
            || match std::fs::metadata(filename) {
                Ok(metadata) => metadata.len() == 0,
                Err(_) => true,
            };
        let mut outfile = std::fs::OpenOptions::new()
            .write(true)
            .append(append)
            .create(true)
            .truncate(!append)
            .open(filename)?;

        if header {
            writeln!(&mut outfile, "{},entity,name,value,timestamp", additional_header)?;
        }

        Ok(outfile)
    }

    ///
    /// Dump the content from Redis to CSV files in `dataset_path`:
    /// - application-metrics.csv
    /// - capabilities.csv
    ///
    pub fn dump_csv(&mut self, dataset_path: &str, append: bool) -> anyhow::Result<()> {
        // Application mettrics.
        let mut outfile = RedisDumper::open_file(
            format!("{}application_metrics.csv", dataset_path).as_str(),
            append,
            &self.additional_header,
        )?;

        for key_in in self.connection.keys::<&str, Vec<String>>("*:*:samples")? {
            let tokens: Vec<&str> = key_in.split(':').collect();
            assert!(tokens.len() == 3);
            let key_out = &tokens[0][0..1].to_string();
            let id = tokens[1];
            self.write_values(&mut outfile, &key_in, key_out, id)?;
        }

        Ok(())
    }

    fn write_values(&mut self, f: &mut std::fs::File, key_in: &str, key_out: &str, name: &str) -> anyhow::Result<()> {
        for value in self.connection.lrange::<&str, Vec<String>>(key_in, 0, -1)? {
            writeln!(f, "{},{},{},{}", self.additional_fields, key_out, name, value)?;
        }
        Ok(())
    }
}
