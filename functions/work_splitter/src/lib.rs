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
}

static STATE: std::sync::OnceLock<std::sync::Mutex<State>> = std::sync::OnceLock::new();

impl EdgeFunction for WorkSplitterFun {
    fn handle_cast(_src: InstanceId, _encoded_message: &[u8]) {
        // TODO: for now we have 9 fixed calculators
        for i in 1..=9 {
            let target = i.to_string();
            let state = STATE.get().unwrap().lock().unwrap();
            let rows = 3;
            let cols = 3;
            let row = (i - 1) / cols;
            let col = (i - 1) % cols;

            let x_step = (state.bottom_right_x - state.top_left_x) / cols as f64;
            let y_step = (state.top_left_y - state.bottom_right_y) / rows as f64; // Note: y_step should be positive since top_left_y > bottom_right_y

            let chunk_top_left_x = state.top_left_x + col as f64 * x_step;
            let chunk_top_left_y = state.top_left_y - row as f64 * y_step; // Subtract because y decreases as we go down
            let chunk_bottom_right_x = chunk_top_left_x + x_step;
            let chunk_bottom_right_y = chunk_top_left_y - y_step; // Subtract y_step to go down

            log::info!(
                "work_splitter: Chunk {} (row={}, col={}): top_left=({:.6}, {:.6}), bottom_right=({:.6}, {:.6})",
                i, row, col, chunk_top_left_x, chunk_top_left_y, chunk_bottom_right_x, chunk_bottom_right_y
            );

            let message = format!(
            "i={},top_left_x={:.6},top_left_y={:.6},bottom_right_x={:.6},bottom_right_y={:.6}",
            i, chunk_top_left_x, chunk_top_left_y, chunk_bottom_right_x, chunk_bottom_right_y
            );
            edgeless_function::cast(&target, &message.into_bytes().as_slice());
        }

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
        let mut top_left_x: f64 = 0.0;
        let mut top_left_y: f64 = 0.0;
        let mut bottom_right_x: f64 = 0.0;
        let mut bottom_right_y: f64 = 0.0;

        if arguments.is_empty() {
            log::error!("work_splitter: no arguments provided in init payload");
            return;
        }
        for (key, value) in arguments.iter() {
            match *key {
                "top_left_x" => top_left_x = value.parse::<f64>().unwrap_or(0.0),
                "top_left_y" => top_left_y = value.parse::<f64>().unwrap_or(0.0),
                "bottom_right_x" => bottom_right_x = value.parse::<f64>().unwrap_or(0.0),
                "bottom_right_y" => bottom_right_y = value.parse::<f64>().unwrap_or(0.0),
                _ => {}
            }
        }
        log::info!(
            "work_splitter: Parsed coordinates: top_left_x={}, top_left_y={}, bottom_right_x={}, bottom_right_y={}",
            top_left_x,
            top_left_y,
            bottom_right_x,
            bottom_right_y
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
        
        let _ = STATE.set(std::sync::Mutex::new(State {
            top_left_x,
            top_left_y,
            bottom_right_x,
            bottom_right_y,
        }));

        // self invoke
        edgeless_function::cast("self", &[]);
    }

    fn handle_stop() {
        log::info!("work_splitter: stopped");
    }
}

edgeless_function::export!(WorkSplitterFun);
