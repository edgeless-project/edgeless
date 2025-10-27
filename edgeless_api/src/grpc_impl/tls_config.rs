use anyhow::Result;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio_rustls::rustls::pki_types::ServerName;
use toml::Value as TomlValue;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint, ServerTlsConfig};

use rustls::{
    client::ResolvesClientCert,
    pki_types::{pem::PemObject, CertificateDer},
    sign::CertifiedKey,
    ClientConfig, RootCertStore, SignatureScheme,
};
use rustls_tpm_signer::{key::TpmKey, signer::TpmSigningKey};

// Static global configurations
static SERVER_TLS_CONFIG: OnceLock<TlsConfig> = OnceLock::new();
static CLIENT_TLS_CONFIG: OnceLock<TlsConfig> = OnceLock::new();

#[derive(Debug, Deserialize, Clone, Default)]
struct CombinedTlsConfig {
    #[serde(default)]
    server: Option<TlsConfig>,
    #[allow(dead_code)]
    #[serde(default)]
    client: Option<TlsConfig>,
}

impl CombinedTlsConfig {
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path.as_ref()).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!("TLS configuration file '{}' not found", path.as_ref().display())
            } else {
                anyhow::anyhow!("Failed to open TLS configuration file '{}': {}", path.as_ref().display(), e)
            }
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let cfg: CombinedTlsConfig = toml::from_str(&contents)?;
        Ok(cfg)
    }
}

#[derive(Debug, Deserialize, Clone)]
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
    /// Loads TLS configuration from the TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path.as_ref()).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!("TLS configuration file '{}' not found", path.as_ref().display())
            } else {
                anyhow::anyhow!("Failed to open TLS configuration file '{}': {}", path.as_ref().display(), e)
            }
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let value: TomlValue = toml::from_str(&contents)?;
        if let Some(table) = value.as_table() {
            if let Some(client) = table.get("client") {
                return Ok(client.clone().try_into()?);
            }
            if let Some(server) = table.get("server") {
                return Ok(server.clone().try_into()?);
            }
        }

        Ok(toml::from_str(&contents)?)
    }

    /// This function creates the Tonic ServerTlsConfig from the given configuration
    pub fn create_server_tls_config(&self) -> Result<Option<ServerTlsConfig>> {
        // Check that required fields are present
        if self.server_cert_path.is_none() || self.server_key_path.is_none() {
            log::info!("No TLS enabled for Server.");
            return Ok(None); // If no cert or key, then no TLS is activated
        }

        let cert_path = self.server_cert_path.as_ref().unwrap();
        let key_path = self.server_key_path.as_ref().unwrap();

        let mut tls_config =
            ServerTlsConfig::new().identity(tonic::transport::Identity::from_pem(std::fs::read(cert_path)?, std::fs::read(key_path)?));

        // If Server CA is specified, set up mTLS
        if let Some(ca_path) = &self.server_ca_path {
            log::info!("Server CA specified: client authentication will be enforced.");
            tls_config = tls_config.client_ca_root(tonic::transport::Certificate::from_pem(std::fs::read(ca_path)?));
        } else {
            log::info!("No Server CA specified: client authentication will NOT be enforced.");
        }

        Ok(Some(tls_config))
    }

    /// This function creates the Tonic ClientTlsConfig from the given configuration
    pub fn create_client_tls_config(&self) -> Result<ClientTlsConfig> {
        let domain = self.domain_name.clone().unwrap_or_else(|| "www.example.com".to_string());

        let mut tls_config = ClientTlsConfig::new().domain_name(domain);
        // Add CA certificate to verify the Server
        if let Some(ca_path) = &self.client_ca_path {
            log::info!("Client CA specified: TLS will be enforced.");
            tls_config = tls_config.ca_certificate(tonic::transport::Certificate::from_pem(std::fs::read(ca_path)?));
        } else {
            log::info!("No Client CA specified: no TLS will be enforced.");
        }

        // Configure client certificate for mTLS
        if let Some(cert_path) = &self.client_cert_path {
            let client_cert = std::fs::read(cert_path)?;

            // Differentiate between TPM or normal file-based key for the client:
            if let Some(_tpm_handle) = &self.tpm_handle {
                log::info!("mTLS with TPM Client enabled.");
            } else if let Some(key_path) = &self.client_key_path {
                log::info!("mTLS Client enabled.");
                let client_key = std::fs::read(key_path)?;
                tls_config = tls_config.identity(tonic::transport::Identity::from_pem(client_cert, client_key));
            }
        }

        Ok(tls_config)
    }

    /// This function create the Tonic channel with the custom TPM integration
    pub async fn create_channel_with_tpm(&self, server_addr: &str) -> Result<Channel> {
        let ca_path = self
            .client_ca_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CA certificate path is required for TPM integration"))?;
        let cert_path = self
            .client_cert_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client certificate path is required for TPM integration"))?;
        let tpm_handle = self
            .tpm_handle
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("TPM handle is required for TPM integration"))?;

        // Parse TPM handle from string to u32
        let tpm_handle_value = if tpm_handle.starts_with("0x") {
            u32::from_str_radix(&tpm_handle[2..], 16)?
        } else {
            tpm_handle.parse::<u32>()?
        };

        // Create basic TLS config for endpoint (this is just for initialization, real TLS is handled by connector)
        let server_root_ca_cert = tonic::transport::Certificate::from_pem(std::fs::read(ca_path)?);
        let domain = self.domain_name.clone().unwrap_or_else(|| "localhost".to_string());
        let tls = ClientTlsConfig::new().domain_name(&domain).ca_certificate(server_root_ca_cert);

        let endpoint = Endpoint::from_shared(server_addr.to_string())?.tls_config(tls)?;

        // Create custom rustls TLS config with TPM integration
        let ssl_conn = make_ssl_conn(ca_path, cert_path, tpm_handle_value);

        let dns_name = ServerName::try_from(domain)?;

        // Create connector with TPM-based authentication
        let connector = tonic_tls::rustls::TlsConnector::new(&endpoint, ssl_conn, dns_name);

        // Connect with custom connector
        let channel = endpoint.connect_with_connector(connector).await?;

        Ok(channel)
    }

    pub async fn create_client_channel(&self, server_addr: &str) -> Result<Channel> {
        match TlsConfig::from_file("tls_config.toml") {
            Ok(cfg) => {
                if cfg.tpm_handle.is_some() && cfg.client_cert_path.is_some() && cfg.client_ca_path.is_some() {
                    log::info!("Creating channel with TPM integration");
                    return cfg.create_channel_with_tpm(server_addr).await;
                }

                let endpoint = Endpoint::from_shared(server_addr.to_string())?;

                if cfg.client_ca_path.is_some() {
                    log::info!("Applying TLS configuration");
                    let tls_config = cfg.create_client_tls_config()?;
                    return Ok(endpoint.tls_config(tls_config)?.connect().await?);
                }
                log::info!("Using plaintext connection (no TLS)");
                Ok(endpoint.connect().await?)
            }
            Err(e) => {
                if e.to_string().contains("not found") {
                    log::warn!("TLS configuration file 'tls_config.toml' not found. Continuing with plaintext connection (no TLS).");
                } else {
                    log::warn!("Failed to load TLS configuration: {}. Continuing with plaintext connection (no TLS).", e);
                }
                let endpoint = Endpoint::from_shared(server_addr.to_string())?;
                Ok(endpoint.connect().await?)
            }
        }
    }

    /// Returns a global server TLS configuration loaded from 'tls_config.toml'
    pub fn global_server() -> &'static TlsConfig {
        SERVER_TLS_CONFIG.get_or_init(|| match CombinedTlsConfig::from_file("tls_config.toml") {
            Ok(cfg) => cfg.server.unwrap_or_else(TlsConfig::default),
            Err(err) => {
                if err.to_string().contains("not found") {
                    log::warn!("TLS configuration file 'tls_config.toml' not found. Using default TLS configuration (no TLS).");
                } else {
                    log::warn!("Failed to load server TLS config: {}. Using default TLS configuration (no TLS).", err);
                }
                TlsConfig::default()
            }
        })
    }

    /// Returns a global client TLS configuration loaded from 'tls_config.toml'
    pub fn global_client() -> &'static TlsConfig {
        CLIENT_TLS_CONFIG.get_or_init(|| match CombinedTlsConfig::from_file("tls_config.toml") {
            Ok(combined) => {
                if let Some(client_cfg) = combined.client {
                    client_cfg
                } else {
                    TlsConfig::default()
                }
            }
            Err(err) => {
                if err.to_string().contains("not found") {
                    log::warn!("TLS configuration file 'tls_config.toml' not found. Using default TLS configuration (no TLS).");
                } else {
                    log::warn!("Failed to load client TLS config: {}. Using default TLS configuration (no TLS).", err);
                }
                TlsConfig::default()
            }
        })
    }
}

#[derive(Debug)]

pub struct ClientCertResolver {
    client_cert_path: PathBuf,
    client_tpm_key_handle: u32,
}

fn get_chain(client_cert_path: &Path, client_tpm_key_handle: u32) -> anyhow::Result<(Vec<CertificateDer<'static>>, TpmSigningKey)> {
    let certificates = load_certs(client_cert_path);
    let mut client_auth_roots = RootCertStore::empty();
    for (_index, ca) in certificates.iter().enumerate() {
        client_auth_roots.add(ca.clone()).unwrap();
    }

    let key = TpmKey {
        handle: client_tpm_key_handle,
    };

    let signing_key = TpmSigningKey::new(key)?;

    Ok((certificates, signing_key))
}

impl ResolvesClientCert for ClientCertResolver {
    fn resolve(&self, _acceptable_issuers: &[&[u8]], sigschemes: &[SignatureScheme]) -> Option<Arc<CertifiedKey>> {
        //This code here loads the private key and certificate chain representing the CLIENT
        let (chain, signing_key) = get_chain(&self.client_cert_path, self.client_tpm_key_handle).ok()?;

        for scheme in signing_key.supported_schemes() {
            if sigschemes.contains(&scheme) {
                return Some(Arc::new(CertifiedKey {
                    cert: chain,
                    key: Arc::new(signing_key),
                    ocsp: None,
                }));
            }
        }
        None
    }

    fn has_certs(&self) -> bool {
        true
    }
}

fn load_certs(filename: &Path) -> Vec<CertificateDer<'static>> {
    CertificateDer::pem_file_iter(filename)
        .expect("cannot open certificate file")
        .map(|result| result.unwrap())
        .collect()
}

fn make_ssl_conn(client_ca_path: &str, client_cert_path: &str, client_tpm_key_handle: u32) -> std::sync::Arc<tokio_rustls::rustls::ClientConfig> {
    let fd = std::fs::File::open(client_ca_path).expect("Failed to open certificate file");
    let mut roots = RootCertStore::empty();

    let mut buf = std::io::BufReader::new(&fd);
    let certs = rustls_pemfile::certs(&mut buf)
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to parse certs");
    roots.add_parsable_certificates(certs.into_iter());

    let mut config = ClientConfig::builder()
        .with_root_certificates(roots) //Verify the SERVER using the Root CA previously provided
        .with_client_cert_resolver(Arc::new(ClientCertResolver {
            client_cert_path: client_cert_path.into(),
            client_tpm_key_handle: client_tpm_key_handle,
        }));
    config.alpn_protocols = vec![b"h2".to_vec()];
    Arc::new(config)
}
