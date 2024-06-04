// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[test]
#[ignore]
fn test_sysinfo() {
    let mut sys = sysinfo::System::new();

    let my_pid = sysinfo::Pid::from_u32(std::process::id());
    println!("my PID is {}", my_pid);

    loop {
        sys.refresh_cpu();
        sys.refresh_memory();
        sys.refresh_process(my_pid);

        let global_cpu = sys.global_cpu_info();
        println!(
            "global cpu brand {} name {} usage {:.0}% frequency {} Hz",
            global_cpu.brand(),
            global_cpu.name(),
            global_cpu.cpu_usage(),
            global_cpu.frequency()
        );

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

        std::thread::sleep(std::cmp::max(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL, std::time::Duration::from_secs(2)));
    }
}
