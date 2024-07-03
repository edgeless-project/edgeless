use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, BaseConsumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::Message;
use std::time::Duration;

fn main() {
    // Kafka consumer configuration
    let mut consumer_config = ClientConfig::new();
    consumer_config.set("bootstrap.servers", "localhost:9092");
    consumer_config.set("group.id", "my-group");
    consumer_config.set("enable.auto.commit", "true");

    // Create a new Kafka consumer
    let consumer: BaseConsumer = consumer_config
        .create()
        .expect("Consumer creation failed");

    // Subscribe to one or more topics
    consumer
        .subscribe(&["test-topic"])
        .expect("Can't subscribe to specified topic");


    // Consume messages
    loop {
        match consumer.poll(Duration::from_millis(100)) {
            Some(Ok(message)) => {
                process_message(&message);
            }
            Some(Err(err)) => {
                eprintln!("Error while receiving message: {:?}", err);
            }
            None => {}
        }
    }
}

fn process_message(message: &BorrowedMessage<'_>) {
    // Handle the consumed message
    let payload = match message.payload_view() {
        Some(Ok(payload)) => payload,
        _ => return,
    };

    let content = std::str::from_utf8(payload).unwrap();
    println!("Received message: {:?}", content);
}
