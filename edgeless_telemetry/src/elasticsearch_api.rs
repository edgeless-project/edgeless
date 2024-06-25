use anyhow::{anyhow, Result};
use elasticsearch::{
    auth::Credentials, http::transport::SingleNodeConnectionPool, http::transport::TransportBuilder, Elasticsearch, IndexParts, SearchParts,
};
use elasticsearch::{http::transport::Transport, BulkOperation, BulkParts};
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use std::error::Error;
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

pub struct ESClient {
    client: Elasticsearch,
}

impl ESClient {
    pub async fn new() -> anyhow::Result<Self> {
        let file_path = "/workspaces/edgeless/es_creds.json";
        let creds = read_creds_from_file(file_path);

        let url = Url::parse(&creds.url)?;
        let credentials = Credentials::Basic(creds.username, creds.password);
        let conn_pool = elasticsearch::http::transport::SingleNodeConnectionPool::new(url);
        let transport = TransportBuilder::new(conn_pool).auth(credentials).build()?;
        let client = Elasticsearch::new(transport);

        let response = client.ping().send().await?;
        if response.status_code().is_success() {
            log::info!("Elasticsearch Connected");
            Ok(Self { client })
        } else {
            Err(anyhow!("Failed to connect to Elasticsearch"))
        }
    }

    pub async fn write_event(
        &mut self,
        event: &crate::telemetry_events::TelemetryEvent,
        event_tags: &std::collections::BTreeMap<String, String>,
    ) -> Result<(), Box<dyn Error>> {
        log::info!("ELASTICSEARCH WRITE EVENT");
        let id = Uuid::new_v4().to_string();
        let timestamp = get_current_timestamp();
        let document = json!({
            "event": format!("{:?}", event),
            "tags": event_tags,
            "timestamp": timestamp
        });
        log::info!("{:#?}", document);

        let index_response = self
            .client
            .index(IndexParts::IndexId(get_index_name(IndexType::Runtime), &id))
            .body(document)
            .send()
            .await?;
        // Perform the bulk operation
        log::info!("Write to index: Response Status Code: {}", index_response.status_code());
        Ok(())
    }
}

fn read_creds_from_file(file_path: &str) -> Creds {
    let contents = fs::read_to_string(file_path).expect("Failed to read the file");
    let creds: Creds = serde_json::from_str(&contents).expect("Failed to parse JSON");
    creds
}

fn get_index_name(index_type: IndexType) -> &'static str {
    match index_type {
        IndexType::Runtime => "edgeless_runtime",
        IndexType::Resources => "edgeless_resources",
    }
}

pub fn get_current_timestamp() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

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

pub async fn es_write_to_index(client: &Elasticsearch, data: Value, index_type: IndexType) -> Result<(), Box<dyn std::error::Error>> {
    let id = Uuid::new_v4().to_string(); //send data to ES endpoint via POST
    let index_response = client
        .index(IndexParts::IndexId(get_index_name(index_type), &id))
        .body(data)
        .send()
        .await?;

    log::info!("Write to index: Response Status Code: {}", index_response.status_code());
    Ok(())
}
