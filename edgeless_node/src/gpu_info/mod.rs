// SPDX-FileCopyrightText: © 2024 Technical University of Crete
// SPDX-License-Identifier: MIT

pub mod jetson;

/// Enum to represent different board types.
/// If you need to support more board types, add them here.
#[derive(Debug, PartialEq)]
enum BoardType {
    Jetson,
    Other,
}

/// Retrieves the GPU temperature based on the board type.
///
/// If the system is a known BoardType, it calls the board-specific implementation.
/// Otherwise, it returns -1.0.
///
/// # Returns
/// * `f32` - The GPU temperature in Celsius, or a negative number if the operation fails.
///           See: each board-specific implementation for the error number (negative value)
pub fn get_gpu_temp() -> f32 {
    match board_type() {
        BoardType::Jetson => jetson::jetson_get_gpu_temp(),
        BoardType::Other => -1.0,
    }
}

/// Retrieves the instantaneous GPU load in percentage based on the board type.
///
/// If the system is a known BoardType, it calls the board-specific implementation.
/// Otherwise, it returns -1.
///
/// # Returns
/// * `i32` - The instantaneous GPU load in percentage [%], or a negative number if the operation fails.
///           See: each board-specific implementation for the error number (negative value)
pub fn get_gpu_load() -> i32 {
    match board_type() {
        BoardType::Jetson => jetson::jetson_get_gpu_load(),
        BoardType::Other => -1,
    }
}

/// Retrieves the number of GPUs that exist on the system
///
/// If the system is a known BoardType, it calls the board-specific implementation.
/// Otherwise, it returns 0.
///
/// # Returns
/// * `i32` - The number of available GPUs in the system, or a negative number if the operation fails.
///           See: each board-specific implementation for the error number (negative value)
pub fn get_num_gpus() -> i32 {
    match board_type() {
        BoardType::Other => 0,
    }
}

/// Retrieves the model name of the GPU
///
/// If the system is a known BoardType, it calls the board-specific implementation.
/// Otherwise, it returns an empty string.
///
/// # Returns
/// * String -  The model name of available GPU in the system or an empy string
pub fn get_model_name_gpu() -> String {
    match board_type() {
        BoardType::Other => "".to_string(),
    }
}

/// Retrieves the GPU memory size in kilobytes
///
/// If the system is a known BoardType, it calls the board-specific implementation.
/// Otherwise, it returns 0.
///
/// # Returns
/// * `i32` - The number of available GPU memory, or a negative number if the operation fails.
///           See: each board-specific implementation for the error number (negative value)
pub fn get_mem_size_gpu() -> i32 {
    match board_type() {
        BoardType::Other => 0,
    }
}


/// Determines the board type.
///
/// # Returns
/// * `BoardType` - The type of board
fn board_type() -> BoardType {
    if jetson::is_jetson_board() {
        BoardType::Jetson
    } else {
        BoardType::Other
    }
}

/// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_gpu_temp() {
        let result = get_gpu_temp();
        println!("GPU Temp: {:?}", result);
    }

    #[test]
    fn test_get_gpu_load() {
        let result = get_gpu_load();
        println!("GPU Load: {:?}", result);
    }

    #[test]
    fn test_get_num_gpus() {
        let result = get_num_gpus();
        println!("Num of GPUs: {:?}", result);
    }

    #[test]
    fn test_get_model_name_gpu() {
        let result = get_model_name_gpu();
        println!("GPU Model name: {:?}", result);
    }

    #[test]
    fn test_get_mem_size_gpu() {
        let result = get_mem_size_gpu();
        println!("GPU mem size: {:?}", result);
    }
}
