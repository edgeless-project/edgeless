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

    /// Remove all the metrics regarding function and workflow.
    pub fn clean_metrics(&mut self) -> redis::RedisResult<()> {
        let patterns = vec!["function", "worflow"];
        for pattern in patterns {
            for key in self.connection.keys::<&str, Vec<String>>(format!("{}:*", pattern).as_str())? {
                let _ = self.connection.del::<&str, usize>(&key);
            }
        }
        Ok(())
    }

    ///
    /// Dump the content from Redis to a CSV file.
    ///
    /// # Example with Pandas
    ///
    /// ```ignore
    /// >>> import pandas as pd
    /// >>> df = pd.read_csv('../../out.csv')
    /// >>> df[df["entity"] == "W"]["value"].mean()
    /// 146.875
    /// >>> df
    ///     entity    name  value
    /// 0        W     wf0    161
    /// 1        W     wf0    151
    /// 2        W     wf0    176
    /// 3        W     wf0    146
    /// 4        W     wf0    121
    /// ..     ...     ...    ...
    /// 120      F  wf1:f3     33
    /// 121      F  wf1:f3     26
    /// 122      F  wf1:f3     30
    /// 123      F  wf1:f3     26
    /// 124      F  wf1:f3     21
    ///
    /// [125 rows x 3 columns]
    /// ```
    pub fn dump_csv(
        &mut self,
        output: &str,
        append: bool,
        workflows: std::collections::HashMap<String, std::collections::HashSet<String>>,
    ) -> anyhow::Result<()> {
        let header = !append
            || match std::fs::metadata(output) {
                Ok(metadata) => metadata.len() == 0,
                Err(_) => true,
            };
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .append(append)
            .create(true)
            .truncate(!append)
            .open(output)?;

        if header {
            writeln!(&mut f, "{},entity,name,value", self.additional_header)?;
        }

        for (workflow, functions) in workflows {
            self.write_values(&mut f, "workflow:latencies", "W", &workflow)?;
            for function in functions {
                self.write_values(&mut f, "function:latencies", "F", format!("{}:{}", workflow, function).as_str())?;
            }
        }

        Ok(())
    }

    fn write_values(&mut self, f: &mut std::fs::File, key_in: &str, key_out: &str, name: &str) -> anyhow::Result<()> {
        for value in self.connection.lrange::<String, Vec<String>>(format!("{}:{}", key_in, name), 0, -1)? {
            writeln!(f, "{},{},{},{}", self.additional_fields, key_out, name, value)?;
        }
        Ok(())
    }
}
