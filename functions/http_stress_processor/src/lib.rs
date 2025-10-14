// SPDX-License-Identifier: MIT
use edgeless_function::*;


// CHANGES

//use std::hint::black_box;
//use std::thread;
//use std::time::{Duration, Instant};

// -------------------------------
// Configuration variables
// Planned to allow their modification via annotations or 'init-payload'
// -------------------------------

const STRESS_MODE: &str = "image";    // "cpu" | "image"

// CPU mode
const STRESS_CPU_MS: u64 = 5_000;      // 5s per cast

// IMAGE mode
const STRESS_IMAGE_W: u32 = 1024;
const STRESS_IMAGE_H: u32 = 1024;
const STRESS_IMAGE_ITERS: usize = 20;
const STRESS_IMAGE_SIGMA: f32 = 2.0;


fn stress_get_mode() -> &'static str { STRESS_MODE }

fn stress_image_get_config() -> (u32, u32, usize, f32) {
    (STRESS_IMAGE_W, STRESS_IMAGE_H, STRESS_IMAGE_ITERS, STRESS_IMAGE_SIGMA)
}


// -------------------------------
// CPU-heavy workload (pure compute, bounded by duration)
// -------------------------------
fn cpu_burn_for_ms(ms: u64) {
    // Aproxima “tiempo” por número de ciclos para evitar usar Instant en WASM.
    // 1 “chunk” ≈ el trabajo de un bloque interior; ajusta el factor si quieres más/menos carga.
    let chunks = (ms as usize).saturating_mul(40);
    let mut acc_f = 0.0f64;
    let mut acc_i: u64 = 0xcbf29ce484222325;
    for _ in 0..chunks {
        for k in 0..20_000 {
            let x = (k as f64) * 0.000_123_456 + acc_f;
            let y = x.sin() * x.cos() + 1.000_000_1;
            acc_f = std::hint::black_box(acc_f + y * x.recip());
        }
        for k in 0..50_000u64 {
            let z = k.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ acc_i.rotate_left(13);
            acc_i = std::hint::black_box(acc_i.wrapping_add(z ^ (k << 1)));
        }
        let _ = std::hint::black_box(is_probably_prime((acc_i as u32 | 1).max(3)));
    }
    log::info!("cpu_burn (wasm/ms) done: acc_f={:.6} acc_i=0x{:x}", acc_f, acc_i);
}

fn is_probably_prime(n0: u32) -> bool {
    if n0 % 2 == 0 { return n0 == 2; }
    let mut d = 3u32;
    let limit = (n0 as f64).sqrt() as u32;
    while d <= limit {
        if n0 % d == 0 { return false; }
        d += 2;
    }
    true
}

fn stress_cpu_run(ms: u64) {
    log::info!("Starting CPU stress for {} ms", ms);
    cpu_burn_for_ms(ms);
    log::info!("CPU stress completed (wasm32 single-thread)");

}


// -------------------------------
// IMAGE workload (in-memory blur loop)
// -------------------------------
mod image_stress {
    use image::{imageops::blur, ImageBuffer, Rgba, RgbaImage};

    pub fn synthetic_rgba(w: u32, h: u32) -> RgbaImage {
        // Generate an image to process. Checkerboard + gradient to avoid trivial kernels
        let mut img: RgbaImage = ImageBuffer::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let g = ((x ^ y) & 0xff) as u8;
                let r = (((x * 13 + y * 7) & 0xff) as u8).wrapping_add((x % 251) as u8);
                let b = g.wrapping_add(((y * 11) & 0xff) as u8);
                img.put_pixel(x, y, Rgba([r, g, b, 255]));
            }
        }
        img
    }

    pub fn image_stress_run(w: u32, h: u32, iters: usize, sigma: f32) {
        log::info!("Starting IMAGE stress: {}x{}, iters={}, sigma={}", w, h, iters, sigma);
        let mut img = synthetic_rgba(w, h);
        for i in 0..iters {
            // blur(&RgbaImage, sigma) -> RgbaImage
            img = blur(&img, sigma);
            if i % 50 == 0 { log::info!("blur iteration {} / {}", i, iters); }
        }
        // Prevent optimization out
        let checksum: u64 = img.pixels().fold(
            0u64,
            |acc, p| acc.wrapping_add((p[0] as u64) << 1 ^ (p[1] as u64) << 2 ^ (p[2] as u64))
        );
        log::info!("IMAGE stress completed, checksum=0x{:x}", checksum);
    }
}


// -------------------------------
// Edge function implementation
// -------------------------------

struct HttpStressProcessor;

impl EdgeFunction for HttpStressProcessor {

    // Called at function instance creation time
    fn handle_init(_payload: Option<&[u8]>, _init_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Method 'Init' called");
    }

    // Called at function instance termination time
    fn handle_stop() {
        log::info!("Method 'Stop' called");
    }

    // Called at asynchronous events without return value
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::info!("Method 'Cast' called, MSG: {:?}", str_message);

        // CHANGED
        let mode = stress_get_mode();
        log::info!("http_stress_processor: 'Cast' running in workload mode='{}'", mode);
        match mode {
            "image" => {
                let (w, h, iters, sigma) = stress_image_get_config();
                image_stress::image_stress_run(w, h, iters, sigma);
            }
            _ => {
                stress_cpu_run(STRESS_CPU_MS);
            }
        }
    
    }

    // Called at synchronous events with return value
    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::info!("Method 'Call' called, MSG: {:?}", str_message);
        edgeless_function::CallRet::NoReply
    }
}

edgeless_function::export!(HttpStressProcessor);
