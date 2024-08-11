use clap::Parser;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::fs::File;
use std::io::{self, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio;
use tokio::time::interval;

/// Structure to hold the command-line arguments
#[derive(Parser)]
struct Args {
    /// Kafka brokers
    #[clap(long, default_value = "localhost:9092")]
    broker: String,

    /// Kafka topic
    #[clap(long, default_value = "test-topic")]
    topic: String,

    /// Duration of the experiments in seconds
    #[clap(long, default_value = "10")]
    duration: u64,

    /// Periodic interval to generate messages in milliseconds
    #[clap(long, default_value = "1000")]
    period: u64,

    /// Name of the output file
    #[clap(long, default_value = "bench.prod")]
    output: String,
}

fn timestamp(instant: &SystemTime) -> String {
    let duration = instant.duration_since(UNIX_EPOCH).unwrap();
    format!("{}.{}", duration.as_secs(), duration.subsec_nanos())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Parse the command-line arguments
    let args = Args::parse();

    // Configurazione del produttore Kafka
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &args.broker)
        .create()
        .expect("Producer creation error");

    let mut counter = 0;

    // Apertura del file di output
    let mut file = File::create(&args.output)?;

     // Scrivi gli argomenti della riga di comando nella prima riga del file
     writeln!(
        file,
        "# ./producer --broker: {} --topic: {} --duration: {} --period: {} --output: {}",
        args.broker, args.topic, args.duration, args.period, args.output
    )?;

    // Durata e intervallo di produzione dei messaggi
    let duration = Duration::from_secs(args.duration); // Durata fissa
    let mut interval = interval(Duration::from_millis(args.period)); // Intervallo periodico

    // Tempo di inizio
    let start_time = SystemTime::now();

    // Produzione di messaggi per la durata fissa
    let end_time = start_time + duration;
    while SystemTime::now() < end_time {
        interval.tick().await;

        let payload = counter.to_string();
        let now = SystemTime::now();
        let ts = timestamp(&now);

        // Invio del messaggio
        let delivery_status = producer
            .send(
                FutureRecord::to(&args.topic)
                    .payload(&payload)
                    .key(&payload),
                tokio::time::Duration::from_secs(0), // Timeout::Never
            )
            .await;

        // Scrittura nel file di output
        match delivery_status {
            Ok((partition, offset)) => {
                writeln!(file, "{} - Payload: {}, Partition: {}, Offset: {}", ts, payload, partition, offset)?;
                println!(
                    "Message delivered to partition [{}] at offset {}",
                    partition, offset
                );
            }
            Err((e, _)) => {
                writeln!(file, "{} - Failed to deliver message: {:?}", ts, e)?;
                eprintln!("Failed to deliver message: {:?}", e);
            }
        }

        counter += 1;
    }

    Ok(())
}
