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

    ///
    /// Dump the content from Redis to a CSV file.
    ///
    /// # Example with Pandas
    ///
    /// ```ignore
    /// >>> import pandas as pd
    /// >>> df = pd.read_csv('out.csv')
    /// >>> df[df["entity"] == "w"]["value"].mean()
    /// 192.82978723404256
    /// >>> df
    ///      seed entity                                  name  value
    /// 0      42      f  2ff32f77-5103-454a-972c-930f416b782c     43
    /// 1      42      f  2ff32f77-5103-454a-972c-930f416b782c     65
    /// 2      42      f  2ff32f77-5103-454a-972c-930f416b782c     43
    /// 3      42      f  2ff32f77-5103-454a-972c-930f416b782c     54
    /// 4      42      f  2ff32f77-5103-454a-972c-930f416b782c     75
    /// ..    ...    ...                                   ...    ...
    /// 234    42      f  16dcb090-077f-4bbf-bd9a-143b85564183     40
    /// 235    42      f  16dcb090-077f-4bbf-bd9a-143b85564183     41
    /// 236    42      f  16dcb090-077f-4bbf-bd9a-143b85564183     41
    /// 237    42      f  16dcb090-077f-4bbf-bd9a-143b85564183     40
    /// 238    42      f  16dcb090-077f-4bbf-bd9a-143b85564183     41
    ///
    /// [239 rows x 4 columns]
    /// ```
    pub fn dump_csv(&mut self, output: &str, append: bool) -> anyhow::Result<()> {
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

        for key_in in self.connection.keys::<&str, Vec<String>>("*:*:samples")? {
            let tokens: Vec<&str> = key_in.split(':').collect();
            assert!(tokens.len() == 3);
            let key_out = &tokens[0][0..1].to_string();
            let id = tokens[1];
            self.write_values(&mut f, &key_in, key_out, id)?;
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
