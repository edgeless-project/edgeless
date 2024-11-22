// SPDX-FileCopyrightText: © 2024 Technical University of Crete
// SPDX-License-Identifier: MIT

use std::fs;
use std::io::{self, BufRead};
use std::path;

/// Jetson-specific implementation to retrieve the instantaneous GPU temperature.
/// see: https://forums.developer.nvidia.com/t/temperatures-for-jetson-nano/181675
/// In order to understand how temp is extracted
///
/// # Returns
/// * `f32` - Returns the temperature in Celsius of the GPU or a negative number if the operation fails
//     NOTE: if is returned:
//         * -25.6
//             The GPU temperature is not available, it is related to the Jetpack used (Jetson AGX Orin)
//             When GPU instantaneous load is bigger than 0, then we can read temp value
//             Read:
//                     https://forums.developer.nvidia.com/t/jetson-agx-orin-faq/237459
//                     https://forums.developer.nvidia.com/t/jtop-doesnt-detect-gpu-temperature-consistently/263895
//
//         * -20.0
//             Could not parse as float the value read from /sys/class/thermal/thermal_zoneX/temp
//
//         * -15.0
//             /sys/class/thermal/ directory does not exist or we cannot read it
//
//         * -10.0
//             Could not read the GPU temperature from /sys/class/thermal/thermal_zoneX/temp file (file does not exist or permission error)
pub fn jetson_get_gpu_temp() -> f32 {
    // We need to check which /sys/class/thermal/thermal_zoneX/ directory contains the GPU temp
    // This variable will be updated in order to hold the thermal_zoneX info
    let mut thermal_zone = "thermal_zoneX".to_string();

    // ------------------------------------------------------------------------------------------------
    // First determine which dir in the /sys/class/thermal/thermal_zoneX/ is related to the GPU
    // By reading the /sys/class/thermal/thermal_zoneX/type and comparing it with the `GPU-temp` string

    // Get the contents of the thermal directory
    let thermal_path = "/sys/class/thermal/".to_string();
    let thermal_files = match fs::read_dir(thermal_path.clone()) {
        Ok(files) => files,
        Err(_) => return -15.0,
    };

    // For each entry in the /sys/class/thermal directory
    for entry in thermal_files {
        // Skip this entry if there's an error reading it
        let entry = match entry {
            Ok(file) => file,
            Err(_) => continue,
        };

        // Skip this entry if the file name is not valid UTF-8
        let file_name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(_) => continue,
        };

        // Get only the thermal_zone files
        if file_name.contains("thermal_zone") {
            // If /sys/class/thermal/${thermal_zone}/type contains "GPU-thermal" then this thermal_zoneX dir contains the GPU temp
            if let Ok(content) = fs::read_to_string(format!("{}{}/type", thermal_path, file_name)) {
                if content.trim() == "GPU-therm" {
                    // Check if the contents contain "GPU-thermal"
                    thermal_zone = file_name; // Set the thermal_zoneX to the current dir since this contains GPU info
                }
            }
        }
    }

    // ------------------------------------------------------------------------------------------------
    // By knowing which directory contains GPU thermal info, we can read it and return the temperature
    // Read /sys/class/thermal/${thermal_zone}/temp file and return the temperature
    match fs::read_to_string(format!("/sys/class/thermal/{}/temp", thermal_zone)) {
        Ok(temp_str) => {
            // Parse the temperature value and return it divided by 1000
            if let Ok(temp) = temp_str.trim().parse::<f32>() {
                temp / 1000.0 // Return temperature in Celsius
            } else {
                -20.0
            }
        }
        _ => -10.0,
    }
}

/// Jetson-specific implementation to retrieve the GPU load percentage.
/// see: https://forums.developer.nvidia.com/t/measure-gpu-load/106980
/// In order to understand how load is extracted
///
/// # Returns
/// * `i32` - The instantaneous GPU load in percentage or a negative number if the operation fails
//     NOTE: if is returned
//         * -20
//             Not possible to parse as i32 the value read from /sys/devices/gpu.{}/load
//
//         * -10
//             Not possible to read the GPU load from /sys/devices/gpu.{}/load file (file does not exist or permission error)
pub fn jetson_get_gpu_load() -> i32 {
    let gpu_load = 0;

    // TODO: in a future version we could also check if multiple /sys/devices/gpu.X/load files exists
    // to choose only one of them

    let load_path = format!("/sys/devices/gpu.{}/load", gpu_load);
    match fs::read_to_string(load_path) {
        Ok(load_str) => {
            // Parse the load value and return it divided by 10
            if let Ok(load) = load_str.trim().parse::<u32>() {
                (load / 10) as i32 // Return load
            } else {
                -20
            }
        }
        _ => -10,
    }
}

/// Jetson-specific implementation to retrieve the model name
/// see: https://forums.developer.nvidia.com/t/get-jetson-model-name-at-runtime/280150
/// In order to understand how it is extracted
///
/// # Returns
/// * `String` - The model name retrieved, or an empty string if an error occurs.
pub fn jetson_get_model_name_gpu() -> String {
    if let Ok(contents) = fs::read_to_string("/sys/firmware/devicetree/base/model") {
        // it also trims all non-printable characters like \0 or \n that may exist at the end
        return contents.chars().filter(|c| c.is_ascii_graphic() || c.is_whitespace()).collect()
    }

    "".to_string() 
}

/// Jetson-specific implementation to retrieve the GPU mem size
/// see: 
/// - https://forums.developer.nvidia.com/t/unified-memory-on-jetson-platforms/187448
/// - https://forums.developer.nvidia.com/t/gpu-memory-usage-got-by-using-cudamemgetinfo-is-different-with-tegrastats-command/243798
///
/// In order to understand why it is extracted like this
///
/// Jetson boards have a Unified Memory, which means that CPU and GPU share the same physical memory
///
/// # Returns
/// * `i32` - The GPU memory size in kb or a negative number if the operation fails
//         * -30.0
//              MemTotal not found in the /proc/meminfo file
//
//         * -20.0
//             Could not parse as u32 the value read from /proc/meminfo 
//
//         * -10.0
//             Could not read the /proc/meminfo file (file does not exist or permission error)
pub fn jetson_get_mem_size_gpu() -> i32 {
    // Read the /proc/meminfo file
    let meminfo = match fs::File::open(path::Path::new("/proc/meminfo")) {
        Ok(f) => f,
        Err(_) => return -10,
    };

    // For each entry in the /proc/meminfo
    for line in io::BufReader::new(meminfo).lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Check if this line contains info regarding the total memory
        if line.starts_with("MemTotal:") {
            // Extract the value and parse it
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(mem_value) = parts.get(1) {
                match mem_value.parse::<u32>() {
                    Ok(mem_total) => {
                        return mem_total as i32;
                    }
                    Err(_) => return -20,
                }
            }
        }
    }

    // MemTotal was not detected in the /proc/meminfo file
    -30
}

/// Jetson-specific implementation to retrieve number of GPUs that the board has.
///
/// # Returns
/// * `i32` - The number of available GPUs or a negative number if the operation fails
pub fn jetson_get_num_gpus() -> i32 {
    1 // TODO: implement this
}


/// Determines if the system is a Jetson board by checking `/sys/firmware/devicetree/base/model`.
///
/// # Returns
/// * `bool` - True if running on a Jetson board, otherwise false.
pub fn is_jetson_board() -> bool {
    get_model_name_gpu().to_lowercase().contains("jetson")
}
