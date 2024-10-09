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
    String::from(
        r##"balancer_id = "2bb0867f-e9ee-4a3a-8872-dbaa5228ee23"
invocation_url = "http://127.0.0.1:7032"
"##,
    )
}
