// Tester: Simulate work_splitter and calculator, write 9 Mandelbrot PNGs to Redis keys 1-9
// Usage: npm start
// Requirements: npm install

import Redis from 'ioredis';
import { PNG } from 'pngjs';

const redis = new Redis();

// Standard HSL to RGB conversion
function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  h = h % 360;
  s = Math.max(0, Math.min(1, s));
  l = Math.max(0, Math.min(1, l));
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  let r = 0, g = 0, b = 0;
  if (h < 60) { r = c; g = x; b = 0; }
  else if (h < 120) { r = x; g = c; b = 0; }
  else if (h < 180) { r = 0; g = c; b = x; }
  else if (h < 240) { r = 0; g = x; b = c; }
  else if (h < 300) { r = x; g = 0; b = c; }
  else { r = c; g = 0; b = x; }
  return [
    Math.round((r + m) * 255),
    Math.round((g + m) * 255),
    Math.round((b + m) * 255)
  ];
}

// Mandelbrot calculation (simple, not optimized)
function mandelbrot(
  width: number,
  height: number,
  upper_left: [number, number],
  lower_right: [number, number],
  max_iter = 255
): Buffer {
  // i here is fixed to 1, only for testing
  console.log(
    `i=1,top_left_x=${upper_left[0]},top_left_y=${upper_left[1]},bottom_right_x=${lower_right[0]},bottom_right_y=${lower_right[1]}`
  );
  const png = new PNG({ width, height });
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      // Map pixel to complex plane
      const re = upper_left[0] + (x / width) * (lower_right[0] - upper_left[0]);
      const im = upper_left[1] + (y / height) * (lower_right[1] - upper_left[1]);
      let zr = 0, zi = 0, iter = 0;
      while (zr * zr + zi * zi <= 4 && iter < max_iter) {
        const tmp = zr * zr - zi * zi + re;
        zi = 2 * zr * zi + im;
        zr = tmp;
        iter++;
      }
      const idx = (y * width + x) << 2;
      let color: [number, number, number];
      if (iter === max_iter) {
        color = [0, 0, 0]; // black for points inside the set
      } else {
        // Color by iteration count (hue cycling)
        color = hslToRgb(360 * iter / max_iter, 1, 0.5);
      }
      png.data[idx] = color[0];
      png.data[idx + 1] = color[1];
      png.data[idx + 2] = color[2];
      png.data[idx + 3] = 255;
    }
  }
  return PNG.sync.write(png);
}

// Interesting Mandelbrot zoom targets (center_x, center_y, scale)
// scale: 1.0 = full set, scale: 0.01 = zoomed in 100 times
const zoomTargets = [
  // Start: full set
  { x: -0.5, y: 0.0, scale: 1.0 },
  // Seahorse Valley
  { x: -0.743643887037151, y: 0.13182590420533, scale: 0.01 },
  // Elephant Valley
  { x: 0.282, y: 0.01, scale: 0.01 },
  // Triple Spiral Valley
  { x: -0.088, y: 0.654, scale: 0.008 },
  // Deep spiral
  { x: -0.743643135, y: 0.1318203125, scale: 0.0002 }, // zoomed in 5000 times
  // Mini Mandelbrot
  { x: -1.25066, y: 0.02012, scale: 0.0005 }
];

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

async function main() {
  const rows = 3, cols = 3;
  const width = 200, height = 200;
  const interval_ms = 200;
  const steps_per_target = 30; // frames between each target
  let step = 0;
  let targetIdx = 0;

  while (true) {
    // Interpolate between current and next target
    const from = zoomTargets[targetIdx % zoomTargets.length];
    const to = zoomTargets[(targetIdx + 1) % zoomTargets.length];
    const t = (step % steps_per_target) / steps_per_target;
    const center_x = lerp(from.x, to.x, t);
    const center_y = lerp(from.y, to.y, t);
    const scale = lerp(from.scale, to.scale, t);

    const view_width = 3.0 * scale;
    const view_height = 2.0 * scale;
    const top_left_x = center_x - view_width / 2;
    const top_left_y = center_y + view_height / 2;
    const bottom_right_x = center_x + view_width / 2;
    const bottom_right_y = center_y - view_height / 2;

    const start = process.hrtime.bigint();

    let loopTimes: bigint[] = [];
    if ((global as any).loopTimes) {
      loopTimes = (global as any).loopTimes;
    } else {
      (global as any).loopTimes = loopTimes;
    }

    for (let i = 1; i <= 9; i++) {
      const row = Math.floor((i - 1) / cols);
      const col = (i - 1) % cols;
      const x_step = (bottom_right_x - top_left_x) / cols;
      const y_step = (bottom_right_y - top_left_y) / rows;
      const chunk_top_left_x = top_left_x + col * x_step;
      const chunk_top_left_y = top_left_y + row * y_step;
      const chunk_bottom_right_x = chunk_top_left_x + x_step;
      const chunk_bottom_right_y = chunk_top_left_y + y_step;

      const maxIter = 500;
      const pngBuf = mandelbrot(
        width,
        height,
        [chunk_top_left_x, chunk_top_left_y],
        [chunk_bottom_right_x, chunk_bottom_right_y],
        maxIter
      );
      const hexPng = pngBuf.toString('hex');
      await redis.set(i.toString(), hexPng);
      // console.log(`Target ${targetIdx}, Frame ${step}: Mandelbrot chunk ${i} to Redis key ${i}`);
    }

    // logging end
    const end = process.hrtime.bigint();
    const durationUs = Number(end - start) / 1000;
    loopTimes.push(BigInt(Math.round(durationUs)));
    const avg = loopTimes.reduce((a, b) => a + b, BigInt(0)) / BigInt(loopTimes.length);
    const std = Math.sqrt(
      loopTimes
        .map(t => Number(t - avg) ** 2)
        .reduce((a, b) => a + b, 0) / loopTimes.length
    );
    console.log(
      `Loop time: ${(durationUs / 1000).toFixed(2)} ms | avg: ${(Number(avg) / 1000).toFixed(2)} ms | std: ${(std / 1000).toFixed(2)} ms`
    );

    step++;

    // advance target every steps_per_target steps (targets are interesting fragments of the mandelbrot set)
    if (step % steps_per_target === 0) {
      targetIdx = (targetIdx + 1) % zoomTargets.length;
    }
    await new Promise(res => setTimeout(res, interval_ms));
  }
}

main().catch(e => { console.error(e); process.exit(1); });
