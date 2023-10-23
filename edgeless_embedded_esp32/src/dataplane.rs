use edgeless_api_core::invocation::InvocationAPI;

pub struct EmbeddedDataplaneHandle {
    pub reg: crate::agent::ResourceRegistry,
}

impl EmbeddedDataplaneHandle {
    pub async fn send(&mut self, slf: edgeless_api_core::instance_id::InstanceId, target: edgeless_api_core::instance_id::InstanceId, msg: &str) {
        let event = edgeless_api_core::invocation::Event::<&[u8]> {
            target: target,
            source: slf,
            stream_id: 0,
            data: edgeless_api_core::invocation::EventData::Cast(msg.as_bytes()),
        };
        self.reg.handle(event).await.unwrap();
    }
}
