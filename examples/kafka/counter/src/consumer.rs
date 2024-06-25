use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use tokio::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use anyhow::Result;

#[derive(Serialize, Deserialize, Debug)]
struct MyMessage {
    number: i32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Configura il consumer
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "my_group")
        .set("bootstrap.servers", "localhost:9092")
        .set("auto.offset.reset", "earliest")
        .create()?;

    // Sottoscrivi al topic
    consumer.subscribe(&["test-topic"])?;

    // Consuma i messaggi dal topic
    let mut message_stream = consumer.start();

    while let Some(result) = message_stream.next().await {
        match result {
            Ok(m) => {
                if let Some(payload) = m.payload() {
                    let message: MyMessage = serde_json::from_slice(payload)?;
                    println!("Received message: {:?}", message);

                    // Elabora il messaggio (raddoppia il numero)
                    let doubled_value = message.number * 2;
                    println!("Doubled value: {}", doubled_value);

                    // Puoi inviare il risultato ad un altro topic se necessario
                }
            }
            Err(e) => {
                eprintln!("Error receiving message: {:?}", e);
            }
        }
    }

    Ok(())
}
