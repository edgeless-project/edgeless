// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_api::outer::node_register::NodeRegisterAPI;
use futures::{Future, SinkExt, StreamExt};
use rand::distributions::Distribution;

#[derive(Clone)]
pub struct NodeSubscriber {
    sender: futures::channel::mpsc::UnboundedSender<NodeSubscriberRequest>,
}

#[derive(Clone)]
pub enum NodeSubscriberRequest {
    Refresh(),
}

impl NodeSubscriber {
    pub async fn new(
        node_register_url: String,
        node_id: uuid::Uuid,
        agent_url: String,
        invocation_url: String,
        resource_providers: Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
        capabilities: edgeless_api::node_registration::NodeCapabilities,
        subscription_refresh_interval_sec: u64,
        telemetry_performance_target: edgeless_telemetry::performance_target::PerformanceTargetInner,
    ) -> (
        Self,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
    ) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let sender_cloned = sender.clone();
        let mut rng = rand::thread_rng();
        let counter = rand::distributions::Uniform::from(0..u64::MAX).sample(&mut rng);

        let main_task = Box::pin(async move {
            Self::main_task(
                node_register_url,
                node_id,
                agent_url,
                invocation_url,
                resource_providers,
                capabilities,
                subscription_refresh_interval_sec,
                counter,
                receiver,
                telemetry_performance_target,
            )
            .await;
        });

        let refresh_task = Box::pin(async move {
            Self::refresh_task(sender_cloned, subscription_refresh_interval_sec).await;
        });

        (Self { sender }, main_task, refresh_task)
    }

    async fn refresh_task(sender: futures::channel::mpsc::UnboundedSender<NodeSubscriberRequest>, subscription_refresh_interval_sec: u64) {
        let mut sender = sender;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(subscription_refresh_interval_sec));
        loop {
            interval.tick().await;
            let _ = sender.send(NodeSubscriberRequest::Refresh()).await;
        }
    }

    async fn main_task(
        node_register_url: String,
        node_id: uuid::Uuid,
        agent_url: String,
        invocation_url: String,
        resource_providers: Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
        capabilities: edgeless_api::node_registration::NodeCapabilities,
        subscription_refresh_interval_sec: u64,
        counter: u64,
        receiver: futures::channel::mpsc::UnboundedReceiver<NodeSubscriberRequest>,
        telemetry_performance_target: edgeless_telemetry::performance_target::PerformanceTargetInner,
    ) {
        let mut receiver = receiver;
        let mut client = edgeless_api::grpc_impl::outer::node_register::NodeRegisterAPIClient::new(node_register_url).await;
        let mut telemetry_performance_target = telemetry_performance_target;

        // Internal data structures to query system/process information.
        let mut sys = sysinfo::System::new();
        if !sysinfo::IS_SUPPORTED_SYSTEM {
            log::warn!(
                "The library sysinfo does not support (yet) this OS: {}",
                sysinfo::System::os_version().unwrap_or(String::from("unknown"))
            );
        }
        let mut networks = sysinfo::Networks::new_with_refreshed_list();
        let mut disks = sysinfo::Disks::new();
        let own_pid = sysinfo::Pid::from_u32(std::process::id());

        while let Some(req) = receiver.next().await {
            match req {
                NodeSubscriberRequest::Refresh() => {
                    log::debug!("Node Subscriber Refresh");
                    // The refresh deadline is set to twice the refresh period
                    // to reduce the likelihood of a race condition on the
                    // register side.
                    let update_node_request = edgeless_api::node_registration::UpdateNodeRequest {
                        node_id: node_id.clone(),
                        invocation_url: invocation_url.clone(),
                        agent_url: agent_url.clone(),
                        resource_providers: resource_providers.clone(),
                        capabilities: capabilities.clone(),
                        refresh_deadline: std::time::SystemTime::now() + std::time::Duration::from_secs(subscription_refresh_interval_sec * 2),
                        counter,
                        health_status: Self::get_health_status(&mut sys, &mut networks, &mut disks, own_pid),
                        performance_samples: edgeless_api::node_registration::NodePerformanceSamples {
                            function_execution_times: telemetry_performance_target.get_metrics().function_execution_times,
                        },
                    };
                    match client.node_registration_api().update_node(update_node_request).await {
                        Ok(response) => {
                            if let edgeless_api::node_registration::UpdateNodeResponse::ResponseError(err) = response {
                                log::error!("Update of node '{}' rejected by node register: {}", node_id, err);
                            }
                        }
                        Err(err) => log::error!("Update of node '{}' failed: {}", node_id, err),
                    };
                }
            }
        }
    }

    fn get_health_status(
        sys: &mut sysinfo::System,
        networks: &mut sysinfo::Networks,
        disks: &mut sysinfo::Disks,
        own_pid: sysinfo::Pid,
    ) -> edgeless_api::node_registration::NodeHealthStatus {
        // Refresh system/process information.
        sys.refresh_all();
        networks.refresh();
        disks.refresh_list();
        disks.refresh();

        let proc = sys.process(own_pid).expect("Cannot find own PID");

        let to_kb = |x| (x / 1024) as i32;
        let load_avg = sysinfo::System::load_average();
        let mut tot_rx_bytes: i64 = 0;
        let mut tot_rx_pkts: i64 = 0;
        let mut tot_rx_errs: i64 = 0;
        let mut tot_tx_bytes: i64 = 0;
        let mut tot_tx_pkts: i64 = 0;
        let mut tot_tx_errs: i64 = 0;
        for (_interface_name, network) in networks.iter() {
            tot_rx_bytes += network.total_received() as i64;
            tot_rx_pkts += network.total_packets_received() as i64;
            tot_rx_errs += network.total_errors_on_received() as i64;
            tot_tx_bytes += network.total_packets_transmitted() as i64;
            tot_tx_pkts += network.total_transmitted() as i64;
            tot_tx_errs += network.total_errors_on_transmitted() as i64;
        }
        let mut disk_tot_reads = 0;
        let mut disk_tot_writes = 0;
        for process in sys.processes().values() {
            let disk_usage = process.disk_usage();
            disk_tot_reads += disk_usage.total_read_bytes as i64;
            disk_tot_writes += disk_usage.total_written_bytes as i64;
        }
        let unique_available_space = disks
            .iter()
            .map(|x| (x.name().to_str().unwrap_or_default(), x.total_space()))
            .collect::<std::collections::BTreeMap<&str, u64>>();
        edgeless_api::node_registration::NodeHealthStatus {
            mem_free: to_kb(sys.free_memory()),
            mem_used: to_kb(sys.used_memory()),
            mem_available: to_kb(sys.available_memory()),
            proc_cpu_usage: proc.cpu_usage() as i32,
            proc_memory: to_kb(proc.memory()),
            proc_vmemory: to_kb(proc.virtual_memory()),
            load_avg_1: (load_avg.one * 100_f64).round() as i32,
            load_avg_5: (load_avg.five * 100_f64).round() as i32,
            load_avg_15: (load_avg.fifteen * 100_f64).round() as i32,
            tot_rx_bytes,
            tot_rx_pkts,
            tot_rx_errs,
            tot_tx_bytes,
            tot_tx_pkts,
            tot_tx_errs,
            disk_free_space: unique_available_space.values().sum::<u64>() as i64,
            disk_tot_reads,
            disk_tot_writes,
            gpu_load_perc: crate::gpu_info::get_gpu_load(),
            gpu_temp_cels: (crate::gpu_info::get_gpu_temp() * 1000.0) as i32,
        }
    }

    pub fn get_subscriber_sender(&mut self) -> futures::channel::mpsc::UnboundedSender<NodeSubscriberRequest> {
        self.sender.clone()
    }
}
