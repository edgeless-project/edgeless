use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
// use sysinfo::System::SystemExt;
use chrono::{DateTime, Utc};
// use std::sync::{Arc, Mutex};
use crate::elasticsearch_api;
use uuid::Uuid;

use sysinfo::*;
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

pub async fn elasticsearch_resources() {
    let mut system = System::new_all();

    let id_generator = Uuid::new_v4().to_string();

    let client = match elasticsearch_api::es_create_client().await {
        Ok(client) => client,
        Err(error) => {
            log::error!("es_create_client {}", error); //Log also the error message
            return;
        }
    };
    //Create index
    let _ = elasticsearch_api::es_create_index(&client, elasticsearch_api::IndexType::Runtime);
    //loop to calculate system resources
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await; // Sleep
                                                                            //retrieve CPU usage for all processors

        // let mut system = system.lock().unwrap();
        system.refresh_all();
        let cpu_usage = system.global_cpu_info().cpu_usage();
        //calculate total CPU usage
        let total_cpu_usage = cpu_usage / system.cpus().len() as f32 * 100.0;

        //print total CPU usage
        // log::info!("Total CPU Usage: {:.5}%", total_cpu_usage);
        let total_memory_usage = system.used_memory() as f32 / system.total_memory() as f32 * 100.0;
        // log::info!("Memory Usage: {:?}%", total_memory_usage);

        //save resources to struct
        let data = SystemResources {
            cpu_percent: total_cpu_usage,
            memory_percent: total_memory_usage,
            timestamp: elasticsearch_api::get_current_timestamp(),
        };
        //convert to json value
        let data_value = convert_to_value(&data);
        //write to index
        let _ = elasticsearch_api::es_write_to_index(&client, data_value, elasticsearch_api::IndexType::Resources).await;
        // log::info!("Written to Elasticsearch");
        // let contents = elasticsearch_api::es_read_from_index(&client, elasticsearch_api::IndexType::Resources).await;
        // log::info!("{:#?}", contents);
    }
}

//TODO: USE ESCLIENT
