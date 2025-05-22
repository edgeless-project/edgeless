// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
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
        msg: &str,
        // created: edgeless_api_core::event_timestamp::EventTimestamp,
    ) {
        let event = edgeless_api_core::invocation::Event::<&[u8]> {
            target,
            source: slf,
            stream_id: 0,
            data: edgeless_api_core::invocation::EventData::Cast(msg.as_bytes()),
            created: edgeless_api_core::event_timestamp::EventTimestamp::default(),
        };
        self.reg.handle(event).await.unwrap();
    }
}
