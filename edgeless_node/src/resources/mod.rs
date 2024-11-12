// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
pub mod dda;
pub mod file_log;
pub mod http_egress;
pub mod http_ingress;
#[cfg(feature = "rdkafka")]
pub mod kafka_egress;
pub mod metrics_collector;
pub mod ollama;
pub mod redis;
pub mod resource_provider_specs;
