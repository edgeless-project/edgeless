// SPDX-FileCopyrightText: © 2025 Miguel Mesa-Simon <miguelangel.mesasimon@infineon.com>
// SPDX-License-Identifier: MIT

#[derive(Debug, serde::Deserialize, Clone)]
pub struct TlsConfig {
    /// Path to the Server's Cert
    pub server_cert_path: Option<String>,
    /// Path to the Server's private key
    pub server_key_path: Option<String>,
    /// Path to the Server's CA for Client verification
    pub server_ca_path: Option<String>,

    /// Path to the client's Cert
    pub client_cert_path: Option<String>,
    /// Path to the Client's private key
    pub client_key_path: Option<String>,
    /// Path to the client's CA for Server verification
    pub client_ca_path: Option<String>,
    /// TPM handle alternative for client private key
    pub tpm_handle: Option<String>,
    /// Optional domain name for TLS verification
    pub domain_name: Option<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            server_cert_path: None,
            server_key_path: None,
            server_ca_path: None,

            client_cert_path: None,
            client_key_path: None,
            client_ca_path: None,

            tpm_handle: None,
            domain_name: Some("www.example.com".to_string()),
        }
    }
}

impl TlsConfig {
    pub fn is_tpm_enabled(&self) -> bool {
        self.tpm_handle.is_some() && self.client_cert_path.is_some() && self.client_ca_path.is_some()
    }
}
