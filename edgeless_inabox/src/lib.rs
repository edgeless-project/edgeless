// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
#[derive(Debug)]
pub struct InABoxConfig {
    pub node_conf_files: Vec<String>,
    pub orc_conf_file: String,
    pub bal_conf_file: String,
    pub con_conf_file: String,
}

pub fn edgeless_inabox_main(
    async_runtime: &tokio::runtime::Runtime,
    async_tasks: &mut Vec<tokio::task::JoinHandle<()>>,
    in_a_box_config: InABoxConfig,
) -> anyhow::Result<()> {
    let mut node_confs: Vec<edgeless_node::EdgelessNodeSettings> = Vec::new();
    for node_conf_file in in_a_box_config.node_conf_files {
        if std::path::Path::new(&node_conf_file).exists() {
            node_confs.push(toml::from_str(&std::fs::read_to_string(node_conf_file)?)?);
        }
    }
    let orc_conf = if std::path::Path::new(&in_a_box_config.orc_conf_file).exists() {
        Some(toml::from_str::<edgeless_orc::EdgelessOrcSettings>(&std::fs::read_to_string(
            in_a_box_config.orc_conf_file,
        )?)?)
    } else {
        None
    };
    let bal_conf = if std::path::Path::new(&in_a_box_config.bal_conf_file).exists() {
        Some(toml::from_str::<edgeless_bal::EdgelessBalSettings>(&std::fs::read_to_string(
            in_a_box_config.bal_conf_file,
        )?)?)
    } else {
        None
    };
    let con_conf = if std::path::Path::new(&in_a_box_config.con_conf_file).exists() {
        Some(toml::from_str::<edgeless_con::EdgelessConSettings>(&std::fs::read_to_string(
            in_a_box_config.con_conf_file,
        )?)?)
    } else {
        None
    };

    log::info!("Starting EDGELESS-in-a-box");

    if let Some(bal_conf) = bal_conf {
        async_tasks.push(async_runtime.spawn(edgeless_bal::edgeless_bal_main(bal_conf)));
    }
    if let Some(orc_conf) = orc_conf {
        async_tasks.push(async_runtime.spawn(edgeless_orc::edgeless_orc_main(orc_conf)));
    }
    if let Some(con_conf) = con_conf {
        async_tasks.push(async_runtime.spawn(edgeless_con::edgeless_con_main(con_conf)));
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    for node_conf in node_confs {
        async_tasks.push(async_runtime.spawn(edgeless_node::edgeless_node_main(node_conf)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start() -> anyhow::Result<()> {
        // create default configuration files
        let mut dir = std::env::temp_dir();
        dir.push("test_start_remove_me");
        println!("temp dir: {:?}", dir);
        if dir.exists() {
            std::fs::remove_dir_all(dir.to_str().unwrap())?;
        }
        std::fs::create_dir_all(dir.to_str().unwrap())?;
        let node_conf = dir.join(std::path::Path::new("node.toml")).to_str().unwrap().to_string();
        let orc_conf = dir.join(std::path::Path::new("orchestrator.toml")).to_str().unwrap().to_string();
        let bal_conf = dir.join(std::path::Path::new("balancer.toml")).to_str().unwrap().to_string();
        let con_conf = dir.join(std::path::Path::new("controller.toml")).to_str().unwrap().to_string();
        println!("node conf: {}", node_conf);
        println!("orc  conf: {}", orc_conf);
        println!("bal  conf: {}", bal_conf);
        println!("con  conf: {}", con_conf);
        edgeless_api::util::create_template(node_conf.as_str(), edgeless_node::edgeless_node_default_conf().as_str())?;
        edgeless_api::util::create_template(orc_conf.as_str(), edgeless_orc::edgeless_orc_default_conf().as_str())?;
        edgeless_api::util::create_template(bal_conf.as_str(), edgeless_bal::edgeless_bal_default_conf().as_str())?;
        edgeless_api::util::create_template(con_conf.as_str(), edgeless_con::edgeless_con_default_conf().as_str())?;

        // start the services, terminate soon after
        let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
        let mut async_tasks = vec![];

        edgeless_inabox_main(
            &async_runtime,
            &mut async_tasks,
            InABoxConfig {
                node_conf_files: vec![node_conf],
                orc_conf_file: orc_conf,
                bal_conf_file: bal_conf,
                con_conf_file: con_conf,
            },
        )?;

        std::thread::sleep(std::time::Duration::from_millis(500));
        async_tasks.clear();

        // clean up test artifacts
        std::fs::remove_dir_all(dir)?;

        Ok(())
    }
}
