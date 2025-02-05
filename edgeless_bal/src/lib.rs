// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessBalSettings {
    pub balancer_id: uuid::Uuid,
    pub invocation_url: String,
}

pub async fn edgeless_bal_main(settings: EdgelessBalSettings) {
    log::info!("Starting Edgeless Balancer");
    log::debug!("Settings: {:?}", settings);
    let _data_plane = edgeless_dataplane::handle::DataplaneProvider::new(settings.balancer_id, settings.invocation_url.clone(), None).await;

    let _ = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            log::debug!("one more second during which I did nothing at all")
        }
    })
    .await;
}

pub fn edgeless_bal_default_conf() -> String {
    let bal_conf = EdgelessBalSettings {
        balancer_id: uuid::Uuid::new_v4(),
        invocation_url: String::from("http://127.0.0.1:7000"),
    };

    toml::to_string(&bal_conf).expect("Wrong")
}
