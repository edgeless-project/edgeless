// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use core::str::FromStr;

pub struct EPaperDisplayInstanceConfiguration {
    header_text: Option<[u8; 128]>,
}

pub trait EPaper {
    fn set_text(&mut self, new_text: &str);
}

pub struct EPaperDisplay {
    pub instance_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub header: Option<[u8; 128]>,
    // pub display: &'static mut dyn EPaper,
    msg_sender: embassy_sync::channel::Sender<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, heapless::String<1500>, 2>,
}

impl EPaperDisplay {
    #[allow(clippy::needless_lifetimes)] // not needless
    async fn parse_configuration<'a>(
        data: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<EPaperDisplayInstanceConfiguration, edgeless_api_core::common::ErrorResponse> {
        if data.class_type == "epaper-display" {
            let mut config: Option<[u8; 128]> = None;
            for (key, val) in data.configuration {
                if key == "header_text" {
                    let mut header_data: [u8; 128] = [0; 128];
                    let mut i: usize = 0;
                    for b in val.bytes() {
                        header_data[i] = b;
                        i += 1;
                        if i == 128 {
                            break;
                        }
                    }
                    config = Some(header_data);
                }
            }

            Ok(EPaperDisplayInstanceConfiguration { header_text: config })
        } else {
            Err(edgeless_api_core::common::ErrorResponse {
                summary: "Wrong Resource ProviderId",
                detail: None,
            })
        }
    }
}

impl crate::resource::Resource for EPaperDisplay {
    fn provider_id(&self) -> &'static str {
        "epaper-display-1"
    }

    fn resource_class(&self) -> &'static str {
        "epaper-display"
    }

    fn outputs(&self) -> &'static [&'static str] {
        &[]
    }

    async fn has_instance(&self, id: &edgeless_api_core::instance_id::InstanceId) -> bool {
        if self.instance_id == Some(*id) {
            return true;
        }
        false
    }

    async fn launch(&mut self, _spawner: embassy_executor::Spawner, _dataplane_handle: crate::dataplane::EmbeddedDataplaneHandle) {}
}

#[embassy_executor::task]
pub async fn display_writer(
    message_receiver: embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, heapless::String<1500>, 2>,
    display: &'static mut dyn EPaper,
) {
    display.set_text("Edgeless\nInitialized");
    loop {
        let new_message = message_receiver.receive().await;
        display.set_text(&new_message);
    }
}

impl EPaperDisplay {
    #[allow(clippy::new_ret_no_self)]
    pub async fn new(
        sender: embassy_sync::channel::Sender<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, heapless::String<1500>, 2>,
    ) -> &'static mut dyn crate::resource::ResourceDyn {
        static SLF_RAW: static_cell::StaticCell<EPaperDisplay> = static_cell::StaticCell::new();
        SLF_RAW.init_with(|| EPaperDisplay {
            header: None,
            instance_id: None,
            msg_sender: sender,
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
                self.msg_sender.send(heapless::String::<1500>::from_str(message).unwrap()).await;
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
            // self.display.set_text("Display\nStopped");
            self.msg_sender.send(heapless::String::<1500>::from_str("Display Stop").unwrap()).await;
            Ok(())
        } else {
            Err(edgeless_api_core::common::ErrorResponse {
                summary: "Wrong Resource InstanceId",
                detail: None,
            })
        }
    }

    #[allow(clippy::needless_lifetimes)]
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

        self.instance_id = Some(edgeless_api_core::instance_id::InstanceId::new(crate::NODE_ID));

        self.header = instance_specification.header_text;

        if let Some(t) = self.header {
            self.msg_sender
                .send(heapless::String::<1500>::from_str(core::str::from_utf8(&t).unwrap()).unwrap())
                .await;
        } else {
            self.msg_sender
                .send(heapless::String::<1500>::from_str("Display\nStarted").unwrap())
                .await;
        }

        Ok(self.instance_id.unwrap())
    }

    async fn patch(
        &mut self,
        _resource_id: edgeless_api_core::resource_configuration::EncodedPatchRequest<'_>,
    ) -> Result<(), edgeless_api_core::common::ErrorResponse> {
        Ok(())
    }
}
