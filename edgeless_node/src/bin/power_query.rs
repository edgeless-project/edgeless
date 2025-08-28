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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    let mut power_info = PowerInfo::new(&args.endpoint, args.outlet_number).await?;
    println!("{} {} {} W", args.endpoint, args.outlet_number, power_info.active_power().await);

    Ok(())
}
