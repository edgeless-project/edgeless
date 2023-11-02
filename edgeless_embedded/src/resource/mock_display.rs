pub struct MockDisplayInstanceConfiguration {}

pub struct MockDisplay {
    pub instance_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub active: bool,
}

impl MockDisplay {
    async fn parse_configuration<'a>(
        data: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<MockDisplayInstanceConfiguration, edgeless_api_core::common::ErrorResponse> {
        if data.provider_id == "mock-display-1" {
            Ok(MockDisplayInstanceConfiguration {})
        } else {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Wrong Resource ProviderId",
                detail: None,
            });
        }
    }

    pub async fn new() -> &'static mut dyn crate::resource::ResourceDyn {
        static_cell::make_static!(MockDisplay {
            instance_id: None,
            active: false
        })
    }
}

impl crate::resource::Resource for MockDisplay {
    fn provider_id(&self) -> &'static str {
        return "mock-display-1";
    }

    async fn has_instance(&self, id: &edgeless_api_core::instance_id::InstanceId) -> bool {
        if self.instance_id == Some(*id) {
            return true;
        }
        false
    }

    async fn launch(&mut self, _spawner: embassy_executor::Spawner, _dataplane_handle: crate::dataplane::EmbeddedDataplaneHandle) {}
}

impl crate::invocation::InvocationAPI for MockDisplay {
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

impl crate::resource_configuration::ResourceConfigurationAPI for MockDisplay {
    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        log::info!("Display Stop");

        if Some(resource_id) == self.instance_id {
            self.instance_id = None;
            Ok(())
        } else {
            Err(edgeless_api_core::common::ErrorResponse {
                summary: "Wrong Resource InstanceId",
                detail: None,
            })
        }
    }

    async fn start<'a>(
        &mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<edgeless_api_core::instance_id::InstanceId, edgeless_api_core::common::ErrorResponse> {
        log::info!("Display Start");

        let _instance_specification = Self::parse_configuration(instance_specification).await?;

        if self.instance_id.is_some() {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Resource Busy",
                detail: None,
            });
        }

        let id = edgeless_api_core::instance_id::InstanceId::new(crate::NODE_ID.clone());

        self.instance_id = Some(id);

        Ok(id)
    }
}
