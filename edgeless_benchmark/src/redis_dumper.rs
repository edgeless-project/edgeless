// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

extern crate redis;
// use redis::{Commands, ConnectionLike};

pub struct RedisDumper {
    connection: redis::Connection,
}

impl RedisDumper {
    pub fn new(redis_url: &str) -> Result<Self, String> {
        match redis::Client::open(redis_url) {
            Ok(client) => match client.get_connection() {
                Ok(val) => Ok(Self { connection: val }),
                Err(err) => Err(format!("could not open a Redis connection: {}", err)),
            },
            Err(err) => Err(format!("could not open a Redis connection: {}", err)),
        }
    }

    pub fn flushdb(&mut self) -> redis::RedisResult<()> {
        let _ = redis::cmd("FLUSHDB").query(&mut self.connection)?;
        Ok(())
    }

    pub fn dump_csv(&mut self, output: &str) -> anyhow::Result<()> {
        // XXX
        Ok(())
    }
}
