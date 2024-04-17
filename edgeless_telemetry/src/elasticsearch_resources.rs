use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
// use sysinfo::System::SystemExt;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use sysinfo::*;

use crate::elasticsearch_api;

#[derive(Debug, Serialize, Deserialize)]
struct SystemResources {
    cpu_percent: f32,
    memory_percent: f32,
    timestamp: DateTime<Utc>,
}

fn convert_to_value(data: &SystemResources) -> Value {
    json!({
        "cpu_percent": data.cpu_percent,
        "memory_percent": data.memory_percent,
        "timestamp": data.timestamp,
    })
}

#[tokio::main]
async fn elasticsearch_resources() {
    let system = Arc::new(Mutex::new(System::new_all()));

    //periodically update the system information
    let system_clone = Arc::clone(&system);
    thread::spawn(move || loop {
        let mut system = system_clone.lock().unwrap();
        system.refresh_all();
        drop(system); // Release the lock before sleeping
        thread::sleep(Duration::from_secs(1)); // Update every second
    });

    let client = elasticsearch_api::es_create_client();
    //Create index
    let _ = elasticsearch_api::es_create_index(&client, true);
    //loop to calculate system resources
    loop {
        thread::sleep(Duration::from_secs(2)); // Sleep
                                               //retrieve CPU usage for all processors
        let system = system.lock().unwrap();
        let cpu_usage = system.global_cpu_info().cpu_usage();

        //calculate total CPU usage
        let total_cpu_usage = cpu_usage / system.cpus().len() as f32 * 100.0;

        //print total CPU usage
        println!("Total CPU Usage: {:.5}%", total_cpu_usage);
        let total_memory_usage = system.used_memory() as f32 / system.total_memory() as f32 * 100.0;
        println!("Memory Usage: {:?}%", total_memory_usage);

        //save resources to struct
        let data = SystemResources {
            cpu_percent: total_cpu_usage,
            memory_percent: total_memory_usage,
            timestamp: elasticsearch_api::get_current_timestamp(),
        };
        //convert to json value
        let data_value = convert_to_value(&data);
        //write to index
        let _ = elasticsearch_api::es_write_to_index(&client, data_value, true).await;
        let contents = elasticsearch_api::es_read_from_index(&client, true).await;
        println!("{:#?}", contents);
    }
}
