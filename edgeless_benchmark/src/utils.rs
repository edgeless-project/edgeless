// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

static MEGA: u64 = 1000000;

pub fn to_seconds(us: u64) -> f64 {
    us as f64 / MEGA as f64
}

pub fn to_microseconds(s: f64) -> u64 {
    (s * MEGA as f64).round() as u64
}

pub fn timestamp_now() -> f64 {
    let duration = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    duration.as_secs_f64()
}
