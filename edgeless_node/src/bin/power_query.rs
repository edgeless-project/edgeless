// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use clap::Parser;
use edgeless_node::power_info::PowerInfo;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("127.0.0.1:5502"))]
    endpoint: String,
    #[arg(short, long, default_value_t = 1)]
    outlet_number: u16,
    /// Print the version number and quit.
    #[arg(long, default_value_t = false)]
    version: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    if args.version {
        println!(
            "{}.{}.{}{}{}",
            env!("CARGO_PKG_VERSION_MAJOR"),
            env!("CARGO_PKG_VERSION_MINOR"),
            env!("CARGO_PKG_VERSION_PATCH"),
            if env!("CARGO_PKG_VERSION_PRE").is_empty() { "" } else { "-" },
            env!("CARGO_PKG_VERSION_PRE")
        );
        return Ok(());
    }

    let mut power_info = PowerInfo::new(&args.endpoint, args.outlet_number).await?;
    println!("{} {} {} W", args.endpoint, args.outlet_number, power_info.active_power().await);

    Ok(())
}
