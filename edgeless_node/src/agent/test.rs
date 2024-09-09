// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[test]
#[ignore]
fn test_sysinfo() {
    let mut sys = sysinfo::System::new();

    let my_pid = sysinfo::Pid::from_u32(std::process::id());
    println!("my PID is {}", my_pid);
    let mut networks = sysinfo::Networks::new_with_refreshed_list();

    let mut disks = sysinfo::Disks::new();

    loop {
        sys.refresh_all();

        let global_cpu = sys.global_cpu_usage();
        println!("global cpu usage: {}%", global_cpu);

        for (i, cpu) in sys.cpus().iter().enumerate() {
            println!(
                "cpu#{} brand {} name {} usage {:.0}% frequency {} Hz",
                i,
                cpu.brand(),
                cpu.name(),
                cpu.cpu_usage(),
                cpu.frequency()
            );
        }
        println!(
            "mem free {} bytes, used {} bytes, total {} bytes, available {} bytes",
            sys.free_memory(),
            sys.used_memory(),
            sys.total_memory(),
            sys.available_memory()
        );

        let proc = sys.process(my_pid).unwrap();
        println!(
            "this proc usage {:.0}% memory {} virtual memory {} bytes",
            proc.cpu_usage(),
            proc.memory(),
            proc.virtual_memory()
        );
        println!("\n");

        let load_avg = sysinfo::System::load_average();
        println!(
            "one minute: {}%, five minutes: {}%, fifteen minutes: {}%",
            load_avg.one, load_avg.five, load_avg.fifteen,
        );

        networks.refresh();

        for (interface_name, network) in &networks {
            println!(
                "{}:\ttotal-received {} B ({} pkts), total-transmitted {} B ({} pkts), total-errors rx {} tx {}",
                interface_name,
                network.total_received(),
                network.total_packets_received(),
                network.total_transmitted(),
                network.total_packets_transmitted(),
                network.total_errors_on_received(),
                network.total_errors_on_transmitted()
            );
        }

        let mut tot_rx_bytes = 0;
        let mut tot_rx_pkts = 0;
        let mut tot_rx_errs = 0;
        let mut tot_tx_bytes = 0;
        let mut tot_tx_pkts = 0;
        let mut tot_tx_errs = 0;
        for (_interface_name, network) in &networks {
            tot_rx_bytes += network.total_received();
            tot_rx_pkts += network.total_packets_received();
            tot_rx_errs += network.total_errors_on_received();
            tot_tx_bytes += network.total_packets_transmitted();
            tot_tx_pkts += network.total_transmitted();
            tot_tx_errs += network.total_errors_on_transmitted();
        }
        println!("ALL:\tRX {} bytes {} pkts {} errs", tot_rx_bytes, tot_rx_pkts, tot_rx_errs);
        println!("ALL:\tTX {} bytes {} pkts {} errs", tot_tx_bytes, tot_tx_pkts, tot_tx_errs);

        disks.refresh_list();
        disks.refresh();

        println!("available disk space {} B", disks.iter().map(|x| x.available_space()).sum::<u64>());
        println!("total     disk space {} B", disks.iter().map(|x| x.total_space()).sum::<u64>());

        let mut tot_disk_reads = 0;
        let mut tot_disk_writes = 0;
        for (_, process) in sys.processes() {
            let disk_usage = process.disk_usage();
            tot_disk_reads += disk_usage.total_read_bytes;
            tot_disk_writes += disk_usage.total_written_bytes;
        }
        println!("total reads from disk {} B", tot_disk_reads);
        println!("total writes to disk {} B", tot_disk_writes);

        std::thread::sleep(std::cmp::max(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL, std::time::Duration::from_secs(2)));
    }
}
