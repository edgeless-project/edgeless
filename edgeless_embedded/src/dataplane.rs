// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use crate::invocation::InvocationAPI;

pub struct EmbeddedDataplaneHandle {
    pub reg: crate::agent::EmbeddedAgent,
}

impl EmbeddedDataplaneHandle {
    pub async fn send(
        &mut self,
        slf: edgeless_api_core::instance_id::InstanceId,
        target: edgeless_api_core::instance_id::InstanceId,
        target_port: edgeless_api_core::port::Port<32>,
        msg: &str,
    ) {
        let event = edgeless_api_core::invocation::Event::<&[u8]> {
            target: target,
            source: slf,
            target_port: target_port,
            stream_id: 0,
            data: edgeless_api_core::invocation::EventData::Cast(msg.as_bytes()),
        };
        self.reg.handle(event).await.unwrap();
    }
}
