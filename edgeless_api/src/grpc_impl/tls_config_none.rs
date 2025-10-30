// SPDX-FileCopyrightText: © 2025 Miguel Mesa-Simon <miguelangel.mesasimon@infineon.com>
// SPDX-License-Identifier: MIT

static DEFAULT_TLS_CONFIG: std::sync::LazyLock<super::tls_config::TlsConfig> = std::sync::LazyLock::new(|| super::tls_config::TlsConfig::default());

impl super::tls_config::TlsConfig {
    pub fn global_server() -> &'static super::tls_config::TlsConfig {
        &DEFAULT_TLS_CONFIG
    }

    pub fn global_client() -> &'static super::tls_config::TlsConfig {
        &DEFAULT_TLS_CONFIG
    }

    pub fn create_server_tls_config(&self) -> anyhow::Result<Option<tonic::transport::ServerTlsConfig>> {
        Ok(None)
    }

    pub fn create_client_tls_config(&self) -> anyhow::Result<tonic::transport::ClientTlsConfig> {
        let domain = self.domain_name.clone().unwrap_or_else(|| "www.example.com".to_string());
        Ok(tonic::transport::ClientTlsConfig::new().domain_name(domain))
    }

    pub async fn create_client_channel(&self, server_addr: &str) -> anyhow::Result<tonic::transport::Channel> {
        let endpoint = tonic::transport::Endpoint::from_shared(server_addr.to_string())?;
        Ok(endpoint.connect().await?)
    }

    pub async fn create_channel_with_tpm(&self, _server_addr: &str) -> anyhow::Result<tonic::transport::Channel> {
        anyhow::bail!("cannot create channel with TPM on OS = macos")
    }
}
