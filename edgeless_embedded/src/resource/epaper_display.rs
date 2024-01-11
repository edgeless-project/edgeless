// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
pub struct EPaperDisplayInstanceConfiguration {
    header_text: Option<[u8; 128]>,
}

pub trait EPaper {
    fn set_text(&mut self, new_text: &str);
}

pub struct EPaperDisplay {
    pub instance_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub header: Option<[u8; 128]>,
    pub display: &'static mut dyn EPaper,
}

impl EPaperDisplay {
    async fn parse_configuration<'a>(
        data: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<EPaperDisplayInstanceConfiguration, edgeless_api_core::common::ErrorResponse> {
        if data.provider_id == "epaper-display-1" {
            let mut config: Option<[u8; 128]> = None;
            for configuration_item in data.configuration {
                if let Some((key, val)) = configuration_item {
                    if key == "header_text" {
                        let mut header_data: [u8; 128] = [0; 128];
                        let mut i: usize = 0;
                        for b in val.bytes() {
                            header_data[i] = b;
                            i = i + 1;
                            if i == 128 {
                                break;
                            }
                        }
                        config = Some(header_data);
                    }
                }
            }

            Ok(EPaperDisplayInstanceConfiguration { header_text: config })
        } else {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Wrong Resource ProviderId",
                detail: None,
            });
        }
    }
}

impl crate::resource::Resource for EPaperDisplay {
    fn provider_id(&self) -> &'static str {
        return "epaper-display-1";
    }

    async fn has_instance(&self, id: &edgeless_api_core::instance_id::InstanceId) -> bool {
        if self.instance_id == Some(*id) {
            return true;
        }
        false
    }

    async fn launch(&mut self, _spawner: embassy_executor::Spawner, _dataplane_handle: crate::dataplane::EmbeddedDataplaneHandle) {}
}

impl EPaperDisplay {
    pub async fn new(display: &'static mut dyn EPaper) -> &'static mut dyn crate::resource::ResourceDyn {
        static_cell::make_static!(EPaperDisplay {
            header: None,
            instance_id: None,
            display: display
        })
    }
}

impl crate::invocation::InvocationAPI for EPaperDisplay {
    async fn handle(
        &mut self,
        event: edgeless_api_core::invocation::Event<&[u8]>,
    ) -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()> {
        if let edgeless_api_core::invocation::EventData::Cast(message) = event.data {
            if let Ok(message) = core::str::from_utf8(message) {
                self.display.set_text(message);
            }
        }

        Ok(edgeless_api_core::invocation::LinkProcessingResult::FINAL)
    }
}

impl crate::resource_configuration::ResourceConfigurationAPI for EPaperDisplay {
    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        log::info!("EPaper Display Stop");

        if Some(resource_id) == self.instance_id {
            self.instance_id = None;
            self.display.set_text("Display\nStopped");
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
        log::info!("Epaper Display Start");

        let instance_specification = Self::parse_configuration(instance_specification).await?;

        if self.instance_id.is_some() {
            return Err(edgeless_api_core::common::ErrorResponse {
                summary: "Resource Busy",
                detail: None,
            });
        }

        self.instance_id = Some(edgeless_api_core::instance_id::InstanceId::new(crate::NODE_ID.clone()));

        self.header = instance_specification.header_text;

        if let Some(t) = self.header {
            self.display.set_text(core::str::from_utf8(&t).unwrap());
        } else {
            self.display.set_text("Display\nStarted");
        }

        Ok(self.instance_id.unwrap())
    }
}
