use embedded_graphics::prelude::*;
use embedded_graphics::draw_target::DrawTarget;

pub struct LilyGoLCD<
    DI: display_interface_parallel_gpio::WriteOnlyDataCommand,
    RST: embedded_hal::digital::OutputPin
> {
    pub display: mipidsi::Display<
        DI,
        mipidsi::models::ST7789,
        RST
    >
}

impl<
DI: display_interface_parallel_gpio::WriteOnlyDataCommand,
RST: embedded_hal::digital::OutputPin
    > edgeless_embedded::resource::epaper_display::EPaper for LilyGoLCD<DI, RST>
{
    fn set_text(&mut self, new_text: &str) {
        log::info!("Set Text");
        self.display.clear(embedded_graphics::pixelcolor::Rgb565::WHITE).unwrap();

        let style = embedded_graphics::mono_font::MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
            .text_color(embedded_graphics::pixelcolor::Rgb565::BLUE)
            .build();

        let text_style = embedded_graphics::text::TextStyleBuilder::new()
            .baseline(embedded_graphics::text::Baseline::Top)
            .build();

        embedded_graphics::text::Text::with_text_style(new_text, embedded_graphics::prelude::Point::new(1, 1), style, text_style).draw(&mut self.display).unwrap();
    }
}