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

const STRESS_MODE: &str = "cpu";    // "cpu" | "image"

// CPU mode
const STRESS_CPU_ITERS: u64 = 300;

// IMAGE mode
const STRESS_IMAGE_W: u32 = 128;
const STRESS_IMAGE_H: u32 = 128;
const STRESS_IMAGE_ITERS: usize = 20;
const STRESS_IMAGE_SIGMA: f32 = 2.0;


// -------------------------------
// CPU-heavy workload (intensive trigonometric and bitwise computations)
// -------------------------------
fn is_probably_prime(n0: u32) -> bool {
    if n0 < 4 { return n0 > 1; }
    if n0 % 2 == 0 { return n0 == 2; }
    let mut d = 3u32;
    let limit = (n0 as f64).sqrt() as u32;
    while d <= limit {
        if n0 % d == 0 { return false; }
        d += 2;
    }
    true
}

fn stress_cpu_run(iters: u64) {
    let mut acc_f = 0.0f64;
    let mut acc_i: u64 = 0xcbf29ce484222325;
    for i in 0..iters {
        let x = (i as f64) * 0.000_123_456 + acc_f;
        let y = x.sin() * x.cos() + 1.000_000_1;
        acc_f += y * x.recip();
    
        let z = i.wrapping_mul(0x9E37_79B9) ^ acc_i.rotate_left(7);
        acc_i = acc_i.wrapping_add(z ^ (i as u64) << 1);
    }

    let _ = is_probably_prime((acc_i as u32) | 1);

    log::info!("CPU stress completed");
}


// -------------------------------
// IMAGE workload (in-memory blur loop)
// -------------------------------
fn stress_image_run(w: u32, h: u32, iters: usize, sigma: f32) {
    let mut img = vec![0f32; w*h];

    for y in 0..h { for x in 0..w {
        // sinusoids to have non-trivial data
        img[y*w+x] = 0.5 + 0.5*((x as f32)*0.11).sin()*((y as f32)*0.07).cos();
    }}

    let radius = (3.0*STRESS_IMAGE_SIGMA).ceil() as usize;
    for _ in 0..STRESS_IMAGE_ITERS {
        // Apply Gaussian blur in-place
        fastblur::gaussian_blur_f32(&mut img, w, h, radius);
    }

    // Calculate a checksum to prevent optimization out
    let mut sum: u64 = 0xcbf29ce484222325;
    for &v in &img { sum ^= (v.to_bits() as u64).rotate_left(13).wrapping_mul(0x100000001b3); }

    log::info!(
        "image_stress_blur done ({}x{}, iters={}, sigma={:.1}) checksum=0x{:x}",
        w,
        h,
        STRESS_IMAGE_ITERS,
        STRESS_IMAGE_SIGMA,
        sum
    );
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

        log::info!("Method 'Cast' running in workload mode='{}'", STRESS_MODE);
        match STRESS_MODE {
            "image" => {
                stress_image_run(
                    STRESS_IMAGE_W,
                    STRESS_IMAGE_H,
                    STRESS_IMAGE_ITERS,
                    STRESS_IMAGE_SIGMA,
                );
            }
            _ => {
                stress_cpu_run(STRESS_CPU_ITERS);
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
