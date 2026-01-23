// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct WorkSplitterFun;

struct State {
    top_left_x: f64,
    top_left_y: f64,
    bottom_right_x: f64,
    bottom_right_y: f64,
    zoom_center_x: f64,
    zoom_center_y: f64,
    zoom_factor: f64,
    iteration: u32,
    initial_width: f64,
    initial_height: f64,
    zooming_in: bool,
    min_zoom_factor: f64,
    max_zoom_factor: f64,
}

static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

impl EdgeFunction for WorkSplitterFun {
    fn handle_cast(_src: InstanceId, _encoded_message: &[u8]) {
        let mut state = STATE.get().unwrap().lock().unwrap();
        
        // Calculate current bounds
        let current_top_left_x = state.top_left_x;
        let current_top_left_y = state.top_left_y;
        let current_bottom_right_x = state.bottom_right_x;
        let current_bottom_right_y = state.bottom_right_y;
        
        // Send work chunks to workers
        for i in 1..=9 {
            let target = i.to_string();
            let rows = 3;
            let cols = 3;
            let row = (i - 1) / cols;
            let col = (i - 1) % cols;

            let x_step = (current_bottom_right_x - current_top_left_x) / cols as f64;
            let y_step = (current_top_left_y - current_bottom_right_y) / rows as f64;

            let chunk_top_left_x = current_top_left_x + col as f64 * x_step;
            let chunk_top_left_y = current_top_left_y - row as f64 * y_step;
            let chunk_bottom_right_x = chunk_top_left_x + x_step;
            let chunk_bottom_right_y = chunk_top_left_y - y_step;

            log::debug!(
                "work_splitter: Chunk {} (row={}, col={}): top_left=({:.10}, {:.10}), bottom_right=({:.10}, {:.10})",
                i, row, col, chunk_top_left_x, chunk_top_left_y, chunk_bottom_right_x, chunk_bottom_right_y
            );

            let message = format!(
                "i={},top_left_x={:.10},top_left_y={:.10},bottom_right_x={:.10},bottom_right_y={:.10}",
                i, chunk_top_left_x, chunk_top_left_y, chunk_bottom_right_x, chunk_bottom_right_y
            );
            edgeless_function::cast(&target, &message.into_bytes().as_slice());
        }
        
        // Apply zoom for next iteration
        state.iteration += 1;
        let zoom_speed = 0.98; // Each iteration shows 98% of previous view (2% zoom per iteration)
        
        let width = current_bottom_right_x - current_top_left_x;
        let height = current_top_left_y - current_bottom_right_y;
        
        // Calculate new zoom factor based on direction
        if state.zooming_in {
            state.zoom_factor *= zoom_speed;
            // Switch to zooming out when we reach the max zoom (smallest factor)
            if state.zoom_factor <= state.max_zoom_factor {
                state.zooming_in = false;
                log::info!("work_splitter: Reached max zoom, switching to zoom out");
            }
        } else {
            state.zoom_factor /= zoom_speed;
            // Switch to zooming in when we reach the min zoom (largest factor, back to start)
            if state.zoom_factor >= state.min_zoom_factor {
                state.zooming_in = true;
                log::info!("work_splitter: Reached min zoom, switching to zoom in");
            }
        }
        
        let new_width = state.initial_width * state.zoom_factor;
        let new_height = state.initial_height * state.zoom_factor;
        
        state.top_left_x = state.zoom_center_x - new_width / 2.0;
        state.top_left_y = state.zoom_center_y + new_height / 2.0;
        state.bottom_right_x = state.zoom_center_x + new_width / 2.0;
        state.bottom_right_y = state.zoom_center_y - new_height / 2.0;
        
        log::info!(
            "work_splitter: Iteration {}, {} (zoom: {:.2e}, factor: {:.6}), bounds: ({:.10}, {:.10}) to ({:.10}, {:.10})",
            state.iteration,
            if state.zooming_in { "zooming in" } else { "zooming out" },
            1.0 / state.zoom_factor,
            state.zoom_factor,
            state.top_left_x,
            state.top_left_y,
            state.bottom_right_x,
            state.bottom_right_y
        );

        // self invoke again
        delayed_cast(500, "self", &[]);
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _init_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("work_splitter: started");
        let arguments = edgeless_function::init_payload_to_args(payload);
        let mut top_left_x: f64 = -2.5;
        let mut top_left_y: f64 = 1.5;
        let mut bottom_right_x: f64 = 1.0;
        let mut bottom_right_y: f64 = -1.5;
        
        // Default zoom target: interesting spiral feature near (-0.7, 0.0)
        let mut zoom_center_x: f64 = -0.7;
        let mut zoom_center_y: f64 = 0.0;

        for (key, value) in arguments.iter() {
            match *key {
                "top_left_x" => top_left_x = value.parse::<f64>().unwrap_or(-2.5),
                "top_left_y" => top_left_y = value.parse::<f64>().unwrap_or(1.5),
                "bottom_right_x" => bottom_right_x = value.parse::<f64>().unwrap_or(1.0),
                "bottom_right_y" => bottom_right_y = value.parse::<f64>().unwrap_or(-1.5),
                "zoom_center_x" => zoom_center_x = value.parse::<f64>().unwrap_or(-0.7),
                "zoom_center_y" => zoom_center_y = value.parse::<f64>().unwrap_or(0.0),
                _ => {}
            }
        }
        log::info!(
            "work_splitter: Initial bounds: top_left=({}, {}), bottom_right=({}, {})",
            top_left_x,
            top_left_y,
            bottom_right_x,
            bottom_right_y
        );
        log::info!(
            "work_splitter: Zoom target: ({}, {})",
            zoom_center_x,
            zoom_center_y
        );
        
        // Validate coordinates
        if bottom_right_x <= top_left_x {
            log::error!("work_splitter: Invalid coordinates - bottom_right_x must be greater than top_left_x");
            return;
        }
        if bottom_right_y >= top_left_y {
            log::error!("work_splitter: Invalid coordinates - bottom_right_y must be less than top_left_y (y increases upward in complex plane)");
            return;
        }
        
        log::info!("work_splitter: Coordinates validated successfully");
        
        let initial_width = bottom_right_x - top_left_x;
        let initial_height = top_left_y - bottom_right_y;
        
        // Configure zoom range: zoom in up to 100x, then zoom back out
        let min_zoom_factor = 1.0;      // Full view (starting point)
        let max_zoom_factor = 0.01;     // 100x zoom in
        
        log::info!(
            "work_splitter: Zoom range configured - min_factor: {}, max_factor: {} ({}x zoom)",
            min_zoom_factor,
            max_zoom_factor,
            min_zoom_factor / max_zoom_factor
        );
        
        let _ = STATE.set(std::sync::Mutex::new(State {
            top_left_x,
            top_left_y,
            bottom_right_x,
            bottom_right_y,
            zoom_center_x,
            zoom_center_y,
            zoom_factor: 1.0,
            iteration: 0,
            initial_width,
            initial_height,
            zooming_in: true,
            min_zoom_factor,
            max_zoom_factor,
        }));

        // self invoke
        edgeless_function::cast("self", &[]);
    }

    fn handle_stop() {
        log::info!("work_splitter: stopped");
    }
}

edgeless_function::export!(WorkSplitterFun);
