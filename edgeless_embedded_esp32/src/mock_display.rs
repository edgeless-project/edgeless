pub struct MockDisplayInstanceConfiguration {}

pub struct MockDisplay {
    pub instance_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub active: bool,
}

impl<'a> crate::resource::Resource<'a, MockDisplayInstanceConfiguration> for MockDisplay {
    fn provider_id(&self) -> &'static str {
        return "mock-display-1";
    }

    async fn has_instance(&self, id: &edgeless_api_core::instance_id::InstanceId) -> bool {
        if self.instance_id == Some(*id) {
            return true;
        }
        false
    }
}

impl edgeless_api_core::invocation::InvocationAPI for MockDisplay {
    async fn handle(
        &mut self,
        event: edgeless_api_core::invocation::Event<&[u8]>,
    ) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        if let edgeless_api_core::invocation::EventData::Cast(message) = event.data {
            if let Ok(message) = core::str::from_utf8(message) {
                log::info!("Display Message: {}", message);
            }
        }

        Ok(edgeless_api_core::invocation::LinkProcessingResult::FINAL)
    }
}

impl<'a> edgeless_api_core::resource_configuration::ResourceConfigurationAPI<'a, MockDisplayInstanceConfiguration> for MockDisplay {
    async fn parse_configuration(
        data: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<MockDisplayInstanceConfiguration, ()> {
        if data.provider_id == "mock-display-1" {
            Ok(MockDisplayInstanceConfiguration {})
        } else {
            Err(())
        }
    }

    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), ()> {
        log::info!("Display Stop");

        if Some(resource_id) == self.instance_id {
            self.instance_id = None;
            Ok(())
        } else {
            Err(())
        }
    }

    async fn start(&mut self, _instance_specification: MockDisplayInstanceConfiguration) -> Result<edgeless_api_core::instance_id::InstanceId, ()> {
        log::info!("Display Start");

        if self.instance_id.is_some() {
            return Err(());
        }

        self.instance_id = Some(edgeless_api_core::instance_id::InstanceId::new(crate::NODE_ID.clone()));

        Ok(self.instance_id.unwrap())
    }
}
