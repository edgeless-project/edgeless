// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use edgeless_embedded::resource::epaper_display::EPaper;
use embedded_graphics::prelude::*;
use epd_waveshare::prelude::WaveshareDisplay;
use epd_waveshare::prelude::*;

pub struct LillyGoEPaper<
    SPI: embedded_hal::spi::SpiDevice,
    BUSY: embedded_hal::digital::InputPin,
    DC: embedded_hal::digital::OutputPin,
    RST: embedded_hal::digital::OutputPin,
    DELAY: embedded_hal::delay::DelayUs,
> {
    pub spi_dev: SPI,
    pub delay: DELAY,
    pub epd: epd_waveshare::epd2in13_lillygo::Epd2in13<SPI, BUSY, DC, RST, DELAY>,
    pub display: epd_waveshare::epd2in13_lillygo::Display2in13,
}

impl<
        SPI: embedded_hal::spi::SpiDevice,
        BUSY: embedded_hal::digital::InputPin,
        DC: embedded_hal::digital::OutputPin,
        RST: embedded_hal::digital::OutputPin,
        DELAY: embedded_hal::delay::DelayUs,
    > edgeless_embedded::resource::epaper_display::EPaper for LillyGoEPaper<SPI, BUSY, DC, RST, DELAY>
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
