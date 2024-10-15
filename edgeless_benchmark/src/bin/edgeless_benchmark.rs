// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use clap::Parser;
use core::cmp::Ordering;
use edgeless_benchmark::arrival_model;
use edgeless_benchmark::engine::Engine;
use edgeless_benchmark::utils;
use edgeless_benchmark::workflow_type::WorkflowType;
use std::collections::BinaryHeap;
use std::time;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    /// URL of the controller
    #[arg(short, long, default_value_t = String::from("http://127.0.0.1:7001"))]
    controller_url: String,
    /// URL of the orchestrator
    #[arg(short, long, default_value_t = String::from("http://127.0.0.1:7011"))]
    orchestrator_url: String,
    /// Address to use to bind servers
    #[arg(short, long, default_value_t = String::from("127.0.0.1"))]
    bind_address: String,
    /// Arrival model, one of {poisson, incremental, incr-and-keep, single}
    #[arg(long, default_value_t = String::from("poisson"))]
    arrival_model: String,
    /// Warmup duration, in s
    #[arg(long, default_value_t = 0.0)]
    warmup: f64,
    /// Duration of the benchmarking experiment, in s
    #[arg(short, long, default_value_t = 30.0)]
    duration: f64,
    /// Average lifetime duration of a workflow, in s
    #[arg(short, long, default_value_t = 5.0)]
    lifetime: f64,
    /// Average inter-arrival between consecutive workflows, in s
    #[arg(short, long, default_value_t = 5.0)]
    interarrival: f64,
    /// Seed to initialize the pseudo-random number generators
    #[arg(short, long, default_value_t = 42)]
    seed: u64,
    /// Workflow type, use "help" to list possible examples.
    #[arg(short, long, default_value_t = String::from("single;functions/noop/function.json;functions/noop/noop.wasm"))]
    wf_type: String,
    /// Location of the single_trigger function.
    #[arg(long, default_value_t = String::from("functions/single_trigger/single_trigger.wasm"))]
    single_trigger_wasm: String,
    /// URL of the Redis server to use for metrics.
    #[arg(short, long, default_value_t = String::from("redis://127.0.0.1:6379/"))]
    redis_url: String,
    /// Path where to save the output CSV datasets. If empty, do not save them.
    #[arg(long, default_value_t = String::from(""))]
    dataset_path: String,
    /// Append to the output dataset files.
    #[arg(long, default_value_t = false)]
    append: bool,
    /// Additional fields recorded in the CSV output file.
    #[arg(long, default_value_t = String::from(""))]
    additional_fields: String,
    /// Header of additional fields recorded in the CSV output file.
    #[arg(long, default_value_t = String::from(""))]
    additional_header: String,
}

#[derive(PartialEq, Eq, Debug)]
#[allow(clippy::enum_variant_names)]
enum Event {
    /// 0: Event time.
    /// 1: Time when the workflow ends.
    WfNew(u64, u64),
    /// 0: Event time.
    /// 1: UUID of the workflow.
    WfEnd(u64, String),
    /// 0: Event time.
    WfExperimentEnd(u64),
}

impl Event {
    fn time(&self) -> u64 {
        match self {
            Self::WfNew(t, _) => *t,
            Self::WfEnd(t, _) => *t,
            Self::WfExperimentEnd(t) => *t,
        }
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        other.time().cmp(&self.time())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    let mut arrival_model = arrival_model::ArrivalModel::new(
        args.arrival_model.as_str(),
        args.warmup,
        args.duration,
        args.seed,
        args.interarrival,
        args.lifetime,
    )?;

    // Parse the worflow type from command line option.
    if args.wf_type.to_lowercase() == "help" {
        for wf_type in WorkflowType::examples() {
            println!("{}", wf_type);
        }
        return Ok(());
    }
    let wf_type = match WorkflowType::new(&args.wf_type) {
        Ok(val) => val,
        Err(err) => {
            return Err(anyhow::anyhow!("invalid workflow type: {}", err));
        }
    };

    // Check that the additional fields, if present, have a consistent header.
    let mut additional_fields = args.additional_fields.split(',').filter(|x| !x.is_empty()).collect::<Vec<&str>>();
    let mut additional_header = args.additional_header.split(',').filter(|x| !x.is_empty()).collect::<Vec<&str>>();
    if additional_fields.len() != additional_header.len() {
        return Err(anyhow::anyhow!(
            "mismatching number of additional fields ({}) vs header ({})",
            additional_fields.len(),
            additional_header.len()
        ));
    }
    let seed = format!("{}", args.seed);
    additional_fields.push(&seed);
    additional_header.push("seed");

    // Start the Redis dumper
    let redis_client = edgeless_benchmark::redis_dumper::RedisDumper::new(&args.redis_url, additional_fields.join(","), additional_header.join(","));
    let redis_client = match redis_client {
        Ok(val) => Some(val),
        Err(err) => {
            log::error!("could not connect to Redis at {}: {}", &args.redis_url, err);
            None
        }
    };

    // Create the engine for the creation/termination of workflows.
    let mut engine = Engine::new(&args.controller_url, wf_type, args.seed + 1000, redis_client).await;

    // event queue
    let mut events = BinaryHeap::new();

    // schedule the first event
    if let Some((arrival_time, end_time)) = arrival_model.next(0_u64) {
        events.push(Event::WfNew(arrival_time, end_time));
    }

    // add the end-of-experiment event
    events.push(Event::WfExperimentEnd(utils::to_microseconds(args.duration)));

    // set up warm-up period configuration
    if args.warmup >= args.duration {
        log::warn!(
            "metrics will not be collected since warm-up period ({} s) >= experiment duration ({} s)",
            args.warmup,
            args.duration
        );
    }
    let single_trigger_workflow_id =
        match edgeless_benchmark::engine::setup_metrics_collector(&mut engine, &args.single_trigger_wasm, args.warmup).await {
            Ok(workflow_id) => workflow_id,
            Err(err) => anyhow::bail!("error when setting up the metrics collector: {} ", err),
        };

    // main experiment loop
    let mut wf_started = 0;
    let mut wf_requested = 0;
    let mut now = 0;
    'outer: loop {
        if let Some(event) = events.pop() {
            // wait until the event
            assert!(event.time() >= now, "{} should be >= {}", event.time(), now);
            if event.time() > now {
                std::thread::sleep(time::Duration::from_micros(event.time() - now));
            }

            // handle the event
            now = event.time();
            match event {
                Event::WfNew(_, workflow_end_time) => {
                    wf_requested += 1;
                    if let Ok(uuid) = engine.start_workflow().await {
                        wf_started += 1;
                        log::info!(
                            "{} new wf created '{}', will end at {} s",
                            utils::to_seconds(now),
                            &uuid,
                            utils::to_seconds(workflow_end_time)
                        );
                        events.push(Event::WfEnd(workflow_end_time, uuid));
                    }
                    if let Some((arrival_time, end_time)) = arrival_model.next(now) {
                        if arrival_time < utils::to_microseconds(args.duration) {
                            events.push(Event::WfNew(arrival_time, end_time));
                        }
                    }
                }
                Event::WfEnd(_, uuid) => {
                    log::info!("{} wf terminated  '{}'", utils::to_seconds(now), &uuid);
                    if !uuid.is_empty() {
                        match engine.stop_workflow(&uuid).await {
                            Ok(_) => {}
                            Err(err) => {
                                panic!("error when stopping a workflow: {}", err);
                            }
                        }
                    }
                }
                Event::WfExperimentEnd(_) => {
                    break 'outer;
                }
            }
        }
    }

    // terminate all workflows that are still active
    for event_type in events.iter() {
        if let Event::WfEnd(_, uuid) = event_type {
            if !uuid.is_empty() {
                log::info!("{} wf terminated  '{}'", utils::to_seconds(now), &uuid);
                match engine.stop_workflow(uuid).await {
                    Ok(_) => {}
                    Err(err) => {
                        panic!("error when stopping a workflow: {}", err);
                    }
                }
            }
        }
    }
    let _ = engine.stop_workflow(&single_trigger_workflow_id).await;

    // dump data collected in Redis
    if !args.dataset_path.is_empty() {
        engine.dump(&args.dataset_path, args.append);
    }

    // output metrics
    let blocking_probability = 1.0 - wf_started as f64 / wf_requested as f64;

    log::info!("workflow requested   = {}", wf_requested);
    log::info!("workflow started     = {}", wf_started);
    log::info!("blocking probability = {}", blocking_probability);

    Ok(())
}
