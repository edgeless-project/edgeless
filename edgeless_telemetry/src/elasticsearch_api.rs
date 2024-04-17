use chrono::{DateTime, Utc};
use elasticsearch::indices::IndicesCreateParts;
use elasticsearch::{
    auth::Credentials, http::transport::SingleNodeConnectionPool, http::transport::TransportBuilder, Elasticsearch, IndexParts, SearchParts,
};
use serde_json::json;
use serde_json::Value;
use url::Url;

//static mut variable to hold the counter
static mut COUNTER: u32 = 0;

/// Establish connection to specified ES endpoint.
/// # Returns
/// A Result indicating success or failure.
pub fn es_create_client() -> Result<Elasticsearch, Box<dyn std::error::Error>> {
    //define ES endpoint configs

    let url = Url::parse("elastic_endpoint_url")?; //contant Panagiotis Antoniou(Aegis) for details
    let credentials = Credentials::Basic("username".into(), "password".into()); //contact Panagiotis Antoniou(Aegis)) for details
    let conn_pool = SingleNodeConnectionPool::new(url);
    let transport = TransportBuilder::new(conn_pool).auth(credentials).build()?;

    Ok(Elasticsearch::new(transport))
}

/// Creates an Elasticsearch index with a specified mapping.
/// # Arguments
/// * `client_result` - the Elasticsearch client or an error.
/// # Returns
/// A Result indicating success or failure.
//To be called once on init.
pub async fn es_create_index(
    client_result: &Result<Elasticsearch, Box<dyn std::error::Error>>,
    index_flag: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    //define mapping
    let client = match client_result {
        Ok(client) => client,
        Err(err) => return Err(err.to_string().into()), // Convert error to String
    };
    //define ES index mapping based on the index_flag value
    let mapping = if index_flag {
        //create edgeless_resources mapping
        json!({
            "mappings": {
                "properties": {
                    "cpu_percent": { "type": "float" },
                    "memory_percent": { "type": "float" },
                    "timestamp": { "type": "date" }
                }
            }
        })
    } else {
        //create edgeless_runtime mapping (currently based on stdout console log)
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
    };

    let create_index_response = client
        .indices()
        .create(IndicesCreateParts::Index(get_index_name(index_flag)))
        .body(mapping)
        .send()
        .await?;

    if create_index_response.status_code().is_success() {
        println!("Index 'edgeless_runtime' created successfully");
    } else {
        println!("Failed to create index 'edgeless_runtime': {}", create_index_response.status_code());
    }
    Ok(())
}

//static counter for indexing identification
fn increment_counter() -> String {
    unsafe {
        COUNTER += 1;
        COUNTER.to_string()
    }
}

/// Writes data to an Elasticsearch index.
/// # Arguments
/// * `client_result` - the Elasticsearch client
/// * `data` - The data to be written to the index.
/// * `index_flag` - which index to write to
/// # Returns
/// A Result indicating success or failure.

pub async fn es_write_to_index(
    client_result: &Result<Elasticsearch, Box<dyn std::error::Error>>,
    data: Value,
    index_flag: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = match client_result {
        Ok(client) => client,
        Err(err) => return Err(err.to_string().into()), // Convert error to String
    };
    let id = increment_counter();
    //send data to ES endpoint via POST
    let index_response = client
        .index(IndexParts::IndexId(get_index_name(index_flag), &id))
        .body(data)
        .send()
        .await?;

    println!("Response Status Code: {}", index_response.status_code());
    Ok(())
}

pub fn get_current_timestamp() -> DateTime<Utc> {
    Utc::now()
}

/// Retrieves data from an Elasticsearch index.
/// # Arguments
/// * `client_result` - the Elasticsearch client
/// * `index_flag` - which index to read   
/// # Returns
/// A Result containing a vector of JSON values representing the contents of the index
pub async fn es_read_from_index(
    client_result: &Result<Elasticsearch, Box<dyn std::error::Error>>,
    index_flag: bool,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let client = match client_result {
        Ok(client) => client,
        Err(err) => return Err(err.to_string().into()), // Convert error to String
    };

    let index = get_index_name(index_flag);
    println!("Contents of index {}", index);

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
        Err("Failed to retrieve index contents".into())
    }
}

/// Determines the index name based on the flag indicating whether to use the resources index.
/// # Arguments
/// * `use_resources_index` - A boolean flag indicating whether to use the resources index (`true`) or the runtime index (`false`).
/// # Returns
/// * Returns a string slice (`&'static str`) representing the index name.
fn get_index_name(use_resources_index: bool) -> &'static str {
    match use_resources_index {
        false => "edgeless_runtime",
        true => "edgeless_resources",
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
