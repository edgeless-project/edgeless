// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use clap::Parser;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("http://127.0.0.1:7101/"))]
    endpoint: String,
}

async fn edgeless_container_function_main(endpoint: String) {
    let (mut container_function, container_function_task) =
        edgeless_container_function::container_function::ContainerFunction::new();
    let server_task =
        edgeless_api::grpc_impl::outer::container_function::GuestAPIFunctionServer::run(
            container_function.get_api_client(),
            endpoint,
        );
    futures::join!(container_function_task, server_task);
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let async_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_all()
        .build()?;
    let async_tasks = vec![async_runtime.spawn(edgeless_container_function_main(args.endpoint))];
    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });

    Ok(())
}
