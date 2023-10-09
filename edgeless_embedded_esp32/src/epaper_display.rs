use embedded_graphics::prelude::*;
use epd_waveshare::prelude::*;

pub struct EPaperDisplayInstanceConfiguration {
    header_text: Option<[u8; 128]>,
}

pub trait EPaper {
    fn set_text(&mut self, new_text: &str);
}

pub struct LillyGoEPaper<
    SPI: hal::prelude::eh1::_embedded_hal_1_spi_SpiDevice,
    BUSY: hal::prelude::eh1::_embedded_hal_1_digital_InputPin,
    DC: hal::prelude::eh1::_embedded_hal_1_digital_OutputPin,
    RST: hal::prelude::eh1::_embedded_hal_1_digital_OutputPin,
    DELAY: hal::prelude::eh1::_embedded_hal_1_delay_DelayUs,
> {
    pub spi_dev: SPI,
    pub delay: DELAY,
    pub epd: epd_waveshare::epd2in13_lillygo::Epd2in13<SPI, BUSY, DC, RST, DELAY>,
    pub display: epd_waveshare::epd2in13_lillygo::Display2in13,
}

impl<
        SPI: hal::prelude::eh1::_embedded_hal_1_spi_SpiDevice,
        BUSY: hal::prelude::eh1::_embedded_hal_1_digital_InputPin,
        DC: hal::prelude::eh1::_embedded_hal_1_digital_OutputPin,
        RST: hal::prelude::eh1::_embedded_hal_1_digital_OutputPin,
        DELAY: hal::prelude::eh1::_embedded_hal_1_delay_DelayUs,
    > EPaper for LillyGoEPaper<SPI, BUSY, DC, RST, DELAY>
{
    fn set_text(&mut self, new_text: &str) {
        self.display.clear(Color::White).unwrap();

        let style = embedded_graphics::mono_font::MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
            .text_color(Color::Black)
            .background_color(Color::White)
            .build();

        let text_style = embedded_graphics::text::TextStyleBuilder::new()
            .baseline(embedded_graphics::text::Baseline::Top)
            .build();

        let _ = embedded_graphics::text::Text::with_text_style(new_text, Point::new(0, 5), style, text_style).draw(&mut self.display);
        self.epd
            .update_and_display_frame(&mut self.spi_dev, &self.display.buffer(), &mut self.delay)
            .unwrap();
    }
}

pub struct EPaperDisplay {
    pub instance_id: Option<edgeless_api_core::instance_id::InstanceId>,
    pub header: Option<[u8; 128]>,
    pub display: &'static mut dyn EPaper,
}

impl<'a> crate::resource::Resource<'a, EPaperDisplayInstanceConfiguration> for EPaperDisplay {
    fn provider_id(&self) -> &'static str {
        return "epaper-display-1";
    }

    async fn has_instance(&self, id: &edgeless_api_core::instance_id::InstanceId) -> bool {
        if self.instance_id == Some(*id) {
            return true;
        }
        false
    }
}

impl edgeless_api_core::invocation::InvocationAPI for EPaperDisplay {
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

impl<'a> edgeless_api_core::resource_configuration::ResourceConfigurationAPI<'a, EPaperDisplayInstanceConfiguration> for EPaperDisplay {
    async fn parse_configuration(
        data: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) -> Result<EPaperDisplayInstanceConfiguration, ()> {
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
            Err(())
        }
    }

    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), ()> {
        log::info!("Display Stop");

        if Some(resource_id) == self.instance_id {
            self.instance_id = None;
            self.display.set_text("Display\nStopped");
            Ok(())
        } else {
            Err(())
        }
    }

    async fn start(&mut self, instance_specification: EPaperDisplayInstanceConfiguration) -> Result<edgeless_api_core::instance_id::InstanceId, ()> {
        log::info!("Display Start");

        if self.instance_id.is_some() {
            return Err(());
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
