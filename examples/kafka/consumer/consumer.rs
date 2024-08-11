use clap::Parser;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::Message;
use std::fs::File;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// Structure to hold the command-line arguments
#[derive(Parser)]
struct Args {
    /// Kafka brokers
    #[clap(long, default_value = "localhost:9092")]
    broker: String,

    /// Kafka topic
    #[clap(long, default_value = "test-topic")]
    topic: String,

    /// Name of the output file
    #[clap(long, default_value = "output.txt")]
    output: String,
}

fn timestamp(instant: &SystemTime) -> String {
    let duration = instant.duration_since(UNIX_EPOCH).unwrap();
    format!("{}.{}", duration.as_secs(), duration.subsec_nanos())
}

fn main() -> io::Result<()> {
    // Parse the command-line arguments
    let args = Args::parse();

    // Kafka consumer configuration
    let mut consumer_config = ClientConfig::new();
    consumer_config.set("bootstrap.servers", &args.broker);
    consumer_config.set("group.id", "my-group");
    consumer_config.set("enable.auto.commit", "true");

    // Create a new Kafka consumer
    let consumer: BaseConsumer = consumer_config
        .create()
        .expect("Consumer creation failed");

    // Subscribe to one or more topics
    consumer
        .subscribe(&[&args.topic])
        .expect("Can't subscribe to specified topic");

    // Open the output file
    let mut file = File::create(&args.output)?;

     // Scrivi gli argomenti della riga di comando nella prima riga del file
     writeln!(
        file,
        "# ./consumer --broker: {} --topic: {} --output: {}",
        args.broker, args.topic, args.output
    )?;

    // Consume messages
    loop {
        match consumer.poll(Duration::from_millis(100)) {
            Some(Ok(message)) => {
                process_message(&message, &mut file)?;
            }
            Some(Err(err)) => {
                eprintln!("Error while receiving message: {:?}", err);
            }
            None => {}
        }
    }
}

fn process_message(message: &BorrowedMessage<'_>, file: &mut File) -> io::Result<()> {
    // Handle the consumed message
    let payload = match message.payload_view() {
        Some(Ok(payload)) => payload,
        _ => return Ok(()),
    };

    let content = std::str::from_utf8(payload).unwrap();
    let now = SystemTime::now();
    let ts = timestamp(&now);

    writeln!(file, "{} - Payload: {}", ts, content)?;

    println!("Received message: {:?}", content);
    Ok(())
}
