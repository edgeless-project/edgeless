use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use elasticsearch::indices::IndicesCreateParts;
use elasticsearch::{
    auth::Credentials, http::transport::SingleNodeConnectionPool, http::transport::TransportBuilder, Elasticsearch, IndexParts, SearchParts,
};
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use std::fs;
use url::Url;
use uuid::Uuid;

#[derive(Debug)]
pub enum IndexType {
    Runtime,
    Resources,
}
#[derive(Deserialize)]
struct Creds {
    username: String,
    password: String,
    url: String,
}
fn read_creds_from_file(file_path: &str) -> Creds {
    let contents = fs::read_to_string(file_path).expect("Failed to read the file");
    let creds: Creds = serde_json::from_str(&contents).expect("Failed to parse JSON");
    creds
}

// / Establish connection to specified ES endpoint.
// / # Returns
// / A Result indicating success or failure.
pub async fn es_create_client() -> Result<Elasticsearch> {
    //define ES endpoint configs
    //read credentials from file
    let file_path = "/workspaces/edgeless/es_creds.json";

    // Read the credentials from the file
    let creds = read_creds_from_file(file_path);

    let url = Url::parse(&creds.url)?;
    let credentials = Credentials::Basic(creds.username, creds.password);
    let conn_pool = SingleNodeConnectionPool::new(url);
    let transport = TransportBuilder::new(conn_pool).auth(credentials).build()?;
    let client = Elasticsearch::new(transport);
    // Perform a ping request to ensure the connection is successful
    let response = client.ping().send().await;

    match response {
        Ok(_) => Ok(client),
        Err(err) => Err(anyhow!("Failed to connect to Elasticsearch: {}", err)),
    }
}

#[async_trait]
pub trait ElasticsearchClient {
    fn write_event(
        &self,
        event: &crate::telemetry_events::TelemetryEvent,
        tags: &std::collections::BTreeMap<String, String>,
    ) -> Result<(), elasticsearch::Error>;
}

pub struct ESClient {
    client: Elasticsearch,
}
/// Establish connection to specified ES endpoint.
/// # Returns
/// A Result indicating success or failure.
impl ESClient {
    pub async fn new() -> anyhow::Result<Self> {
        let file_path = "/workspaces/edgeless/es_creds.json";
        let creds = read_creds_from_file(file_path);

        let url = Url::parse(&creds.url)?;
        let credentials = Credentials::Basic(creds.username, creds.password);
        let conn_pool = SingleNodeConnectionPool::new(url);
        let transport = TransportBuilder::new(conn_pool).auth(credentials).build()?;
        let client = Elasticsearch::new(transport);

        let response = client.ping().send().await?;
        if response.status_code().is_success() {
            log::info!("Elasticsearch Connected");
            Ok(Self { client })
        } else {
            Err(anyhow::anyhow!("Failed to connect to Elasticsearch"))
        }
    }
    pub fn write_event(
        &mut self,
        event: &crate::telemetry_events::TelemetryEvent,
        event_tags: &std::collections::BTreeMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = "/workspaces/edgeless/es_creds.json";
        let creds = read_creds_from_file(file_path);
        //synchronous Elasticsearch operations
        log::info!("ELASTICSEARCH WRITE EVENT");
        let id = Uuid::new_v4().to_string();
        let document = json!({
            "event": format!("{:?}", event),
            "tags": event_tags,
        });
        let body = format!(
            "{}\n{}\n",
            json!({ "index": { "_index": "edgeless_runtime", "_id": id } }).to_string(),
            document.to_string()
        );
        let response = self
            .client
            .send(creds.url)
            .header("Content-Type", "application/x-ndjson")
            .basic_auth(creds.username, Some(creds.password))
            .body(body)
            .send()?;

        if response.status().is_success() {
            println!("Bulk operation succeeded");
            Ok(())
        } else {
            println!("Bulk operation failed: {:?}", response);
            Err("Bulk operation failed".into()) // Replace with appropriate error handling
        }
    }
}
/// Creates an Elasticsearch index with a specified mapping.
/// # Arguments
/// * `client_result` - the Elasticsearch client or an error.
/// # Returns
/// A Result indicating success or failure.
//To be called once on init.
pub async fn es_create_index(client: &Elasticsearch, index_type: IndexType) -> Result<(), Box<dyn std::error::Error>> {
    //define mapping
    let mapping = match index_type {
        IndexType::Resources =>
        //create edgeless_runtime mapping (currently based on stdout console log)
        {
            json!({
                "mappings": {
                    "properties": {
                        "cpu_percent": { "type": "float" },
                        "memory_percent": { "type": "float" },
                        "timestamp": { "type": "date" }
                    }
                }
            })
        }
        IndexType::Runtime =>
        //create edgeless_resources mapping
        {
            json!({
                "mappings": {
                    "properties": {
                        "event": { "type": "keyword" },
                        "timestamp": {"type": "date"},
                        "tags": {
                            "properties": {
                                "FUNCTION_ID": { "type": "keyword" },
                                "FUNCTION_TYPE": { "type": "keyword" },
                                "NODE_ID": { "type": "keyword" }
                            }
                        }
                    }
                }
            })
        }
    };

    let create_index_response = client
        .indices()
        .create(IndicesCreateParts::Index(get_index_name(index_type)))
        .body(mapping)
        .send()
        .await?;

    if create_index_response.status_code().is_success() {
        log::info!("Index 'edgeless_runtime' created successfully");
    } else {
        log::error!("Failed to create index 'edgeless_runtime': {}", create_index_response.status_code());
    }
    Ok(())
}

//static counter for indexing identification

/// Writes data to an Elasticsearch index.
/// # Arguments
/// * `client_result` - the Elasticsearch client
/// * `data` - The data to be written to the index.
/// * `index_type: IndexType` - which index to write to
/// # Returns
/// A Result indicating success or failure.

pub async fn es_write_to_index(client: &Elasticsearch, data: Value, index_type: IndexType) -> Result<(), Box<dyn std::error::Error>> {
    // log::info!("called");
    let id = Uuid::new_v4().to_string(); //send data to ES endpoint via POST
    let index_response = client
        .index(IndexParts::IndexId(get_index_name(index_type), &id))
        .body(data)
        .send()
        .await?;

    log::info!("Write to index: Response Status Code: {}", index_response.status_code());
    Ok(())
}

pub fn get_current_timestamp() -> DateTime<Utc> {
    Utc::now()
}

/// Retrieves data from an Elasticsearch index.
/// # Arguments
/// * `client_result` - the Elasticsearch client
/// * `index_type: IndexType` - which index to read   
/// # Returns
/// A Result containing a vector of JSON values representing the contents of the index
pub async fn es_read_from_index(client: &Elasticsearch, index_type: IndexType) -> anyhow::Result<Vec<Value>> {
    let index = get_index_name(index_type);
    log::info!("Contents of index {}", index);

    //fetch data from ES endpoint with index via GET
    let search_response = client
        .search(SearchParts::Index(&[index]))
        .body(json!({
            "query": {
                "match_all": {}
            }
        }))
        .send()
        .await?;
    if search_response.status_code().is_success() {
        let hits = search_response.json::<Value>().await?["hits"]["hits"]
            .as_array()
            .map_or_else(Vec::new, |hits| hits.clone());
        let contents: Vec<Value> = hits.iter().map(|hit| hit["_source"].clone()).collect();
        Ok(contents)
    } else {
        log::error!("Failed to retrieve index contents");
        Err(anyhow::anyhow!("read from index failed")) //TODO Actual error handling
    }
}

/// Determines the index name based on the flag indicating whether to use the resources index.
/// # Arguments
/// * `use_resources_index` - A boolean flag indicating whether to use the resources index (`true`) or the runtime index (`false`).
/// # Returns
/// * Returns a string slice (`&'static str`) representing the index name.
fn get_index_name(index_type: IndexType) -> &'static str {
    match index_type {
        IndexType::Runtime => "edgeless_runtime",
        IndexType::Resources => "edgeless_resources",
    }
}

//FOR TESTING PURPOSES
pub fn es_generate_data() -> Value {
    //get data from edgeless_runtime (dummy atm)
    let event = "FunctionInit(225.227µs)";
    let function_id = "044fa833-a2cc-4201-8177-d46afd53e5ca";
    let function_type = "RUST_WASM";
    let node_id = "fda6ce79-46df-4f96-a0d2-456f720f606c";
    let current_timestamp = get_current_timestamp();

    //construct tags object
    let tags = json!({
        "FUNCTION_ID": function_id,
        "FUNCTION_TYPE": function_type,
        "NODE_ID": node_id
    });

    //construct the data object
    let data = json!({
        "Event": event,
        "timestamp": current_timestamp,
        "tags": tags
    });
    data
}
