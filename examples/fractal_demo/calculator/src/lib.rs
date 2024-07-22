// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
// Fractal calculation code imported and adapted from
// https://github.com/ProgrammingRust/mandelbrot/blob/master/src/main.rs
// (also under MIT license)
use edgeless_function::*;

use colorgrad::Color;
use hex;
use image;
use num::Complex;

struct CalculatorFun;

impl EdgeFunction for CalculatorFun {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::info!("calculator: called with '{}'", str_message);

        let tokens: Vec<&str> = str_message.split(",").collect();
        if tokens.len() == 6 {
            if let Ok(width) = tokens[0].parse::<usize>() {
                if let Ok(height) = tokens[1].parse::<usize>() {
                    if let Ok(top_left_x) = tokens[2].parse::<f64>() {
                        if let Ok(top_left_y) = tokens[3].parse::<f64>() {
                            if let Ok(lower_right_x) = tokens[4].parse::<f64>() {
                                if let Ok(lower_right_y) = tokens[5].parse::<f64>() {
                                    // TODO: work on a simple chunk of pixel data, not on the entire image

                                    let bounds: (usize, usize) = (width, height);
                                    let upper_left = num::complex::Complex::new(top_left_x, top_left_y);
                                    let lower_right = num::complex::Complex::new(lower_right_x, lower_right_y);
                                    let gradient = colorgrad::rainbow();
                                    let colors = gradient.colors(255);
                                    let bytes_per_pixel = image::ColorType::Rgb8.bytes_per_pixel() as usize;
                                    let mut pixels = vec![0; bounds.0 * bounds.1 * bytes_per_pixel];

                                    // Scope of slicing up `pixels` into horizontal bands.
                                    {
                                        log::info!("calculator: preparing");
                                        let bands: Vec<(usize, &mut [u8])> = pixels.chunks_mut(bounds.0 * bytes_per_pixel).enumerate().collect();

                                        log::info!("calculator: calculating");
                                        bands.into_iter().for_each(|(i, band)| {
                                            let top = i;
                                            let band_bounds = (bounds.0, 1);
                                            let band_upper_left = pixel_to_point(bounds, (0, top), upper_left, lower_right);
                                            let band_lower_right = pixel_to_point(bounds, (bounds.0, top + 1), upper_left, lower_right);
                                            render(band, bytes_per_pixel, band_bounds, band_upper_left, band_lower_right, &colors);
                                        });
                                    }

                                    let mut png_data = std::io::Cursor::new(Vec::new());
                                    if image::write_buffer_with_format(
                                        &mut png_data,
                                        &pixels,
                                        bounds.0 as u32,
                                        bounds.1 as u32,
                                        image::ColorType::Rgb8,
                                        image::ImageFormat::Png,
                                    )
                                    .is_ok()
                                    {
                                        log::info!("calculator: saving result as PNG to data buffer suceeded");
                                        let encoded_png_data = hex::encode(png_data.into_inner());
                                        cast("chunk-0-data", &encoded_png_data.as_bytes());
                                    } else {
                                        log::info!("calculator: saving result as PNG to data buffer failed!");
                                    }
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
        log::error!("calculator: error parsing input string");
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _init_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("calculator: started");
    }

    fn handle_stop() {
        log::info!("calculator: stopped");
    }
}

edgeless_function::export!(CalculatorFun);

/// Try to determine if `c` is in the Mandelbrot set, using at most `limit`
/// iterations to decide.
///
/// If `c` is not a member, return `Some(i)`, where `i` is the number of
/// iterations it took for `c` to leave the circle of radius two centered on the
/// origin. If `c` seems to be a member (more precisely, if we reached the
/// iteration limit without being able to prove that `c` is not a member),
/// return `None`.
fn escape_time(c: Complex<f64>, limit: usize) -> Option<usize> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
        z = z * z + c;
    }

    None
}

/// Given the row and column of a pixel in the output image, return the
/// corresponding point on the complex plane.
///
/// `bounds` is a pair giving the width and height of the image in pixels.
/// `pixel` is a (column, row) pair indicating a particular pixel in that image.
/// The `upper_left` and `lower_right` parameters are points on the complex
/// plane designating the area our image covers.
fn pixel_to_point(bounds: (usize, usize), pixel: (usize, usize), upper_left: Complex<f64>, lower_right: Complex<f64>) -> Complex<f64> {
    let (width, height) = (lower_right.re - upper_left.re, upper_left.im - lower_right.im);
    Complex {
        re: upper_left.re + pixel.0 as f64 * width / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64, // Why subtraction here? pixel.1 increases as we go down,
                                                                       // but the imaginary component increases as we go up.
    }
}

/// Render a rectangle of the Mandelbrot set into a buffer of pixels.
///
/// The `bounds` argument gives the width and height of the buffer `pixels`,
/// which holds one grayscale pixel per byte. The `upper_left` and `lower_right`
/// arguments specify points on the complex plane corresponding to the upper-
/// left and lower-right corners of the pixel buffer.
fn render(
    pixels: &mut [u8],
    bytes_per_pixel: usize,
    bounds: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
    colors: &Vec<Color>,
) {
    assert!(pixels.len() == bounds.0 * bounds.1 * 3);

    for row in 0..bounds.1 {
        for column in 0..bounds.0 {
            let point = pixel_to_point(bounds, (column, row), upper_left, lower_right);
            let value: usize = match escape_time(point, 255) {
                None => 0,
                Some(count) => (255 - count) as usize,
            };
            let rgba = colors[value].to_rgba8();
            pixels[row * bounds.0 * bytes_per_pixel + column * bytes_per_pixel] = rgba[0];
            pixels[row * bounds.0 * bytes_per_pixel + column * bytes_per_pixel + 1] = rgba[1];
            pixels[row * bounds.0 * bytes_per_pixel + column * bytes_per_pixel + 2] = rgba[2];
        }
    }
}
