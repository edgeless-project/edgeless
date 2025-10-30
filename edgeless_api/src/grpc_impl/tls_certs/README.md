
# TLS and mTLS configuration
You can secure all network traffic with TLS, mutual TLS (mTLS), or mTLS using a TPM-backed client key. This section explains how to choose a mode, configuring it, and how to set the correct endpoints.

## Configuration file location:
- Each running component (Controller, Orchestrator, Node) reads a `tls_config.toml` file.
- Place `tls_config.toml` next to the binary you're going to run (typically target/debug or target/release).

The expected fields in `tls_config.toml` are:

```
[server]
server_cert_path = ""  # PEM-encoded server certificate (or chain)
server_key_path  = ""  # PEM-encoded private key corresponding to server_cert_path
server_ca_path   = ""  # CA used by the server to verify client certs (mTLS only)

[client]
client_cert_path = ""  # PEM-encoded client certificate (mTLS only)
client_key_path  = ""  # PEM-encoded private key for client_cert_path (only uncomment for mTLS without TPM)
client_ca_path   = ""  # CA used by the client to verify the server certificate
domain_name      = ""  # Server hostname for TLS SNI/verification
tpm_handle       = ""  # TPM handle for the client private key (only uncomment for mTLS with TPM only)
```

A summary of which fields are required for each (m)TLS mode is as follows:
- No TLS (plaintext):
  - Endpoints in `controller/orchestrator/node.toml` must use HTTP.
  - You can omit tls_config.toml or leave all fields empty/commented.
- TLS (server authentication only):
  - Required: server_cert_path, server_key_path, client_ca_path, domain_name
  - Do not set: server_ca_path, client_cert_path, client_key_path, tpm_handle
  - Endpoints in `controller/orchestrator/node.toml` must use HTTPs.
- mTLS (software keys/certificates):
  - Required: server_cert_path, server_key_path, server_ca_path, client_ca_path, client_cert_path, client_key_path and domain_name
  - Do not set: tpm_handle
  - Endpoints in `controller/orchestrator/node.toml` must use https.
- mTLS with TPM (client key in TPM):
  - Required: server_cert_path, server_key_path, server_ca_path, client_ca_path, client_cert_path, tpm_handle.
  - Do not set: client_key_path (since the private key is stored in the TPM).
  - Endpoints in `controller/orchestrator/node.toml` must use http (see “Important: Tonic and custom resolver” below).

## Notes
- Paths can be absolute or relative to the process working directory.
- If you use a private CA, ensure client_ca_path and server_ca_path include the correct CA chain.
- client_ca_path is the trust store the client uses to verify the server. server_ca_path is the trust store the server uses to verify client certificates in mTLS.
- domain_name is used for TLS SNI and hostname verification. It should match the certificate’s subject alternative name.

## Important: Tonic and custom resolver
- Tonic’s built-in TLS resolver only works when the endpoint scheme is HTTPs. If you configure HTTPs but do not provide valid certs/keys per the selected mode, connections will fail.
- Tonic does not natively support TPM-backed client keys for mTLS. To use a TPM for client authentication, you must use the custom TLS resolver. With the custom resolver:
  - Your endpoints must use HTTP so Tonic does not attempt its own TLS.
  - The custom resolver will handle the secure channel and TPM-backed authentication outside of Tonic’s built-in TLS.

TL;DR:
- Use HTTPS endpoints for TLS or mTLS with software keys (Tonic’s TLS).
- Use HTTP endpoints for mTLS with TPM (custom resolver).

## Common pitfalls and troubleshooting
- Handshake fails immediately: the endpoint scheme likely doesn’t match the mode (https required for Tonic TLS; http required for TPM/custom resolver).
- Unknown CA or certificate verify failed: ensure client_ca_path and server_ca_path contain the correct CA chain.
- Hostname mismatch: set domain_name to the server’s DNS name and ensure it’s in the certificate’s SANs.
- Empty or missing paths: any required field for your mode must point to an existing, readable PEM file.
- TPM mode: ensure tpm_handle is correct and the process has permission to access the TPM.

## Example mTLS configuration
The strongest configuration regarding TLS involves mTLS between Controller and Orchestrator, and mTLS with TPM between the Node and the Orchestrator. You can find the configuration files and certificates needed to test this setup in the [tls_certs](https://github.com/edgeless-project/edgeless/tree/main/edgeless_api/src/grpc_impl/tls_certs/) folder. Simply place the corresponding `tls_config.toml` in the `target/debug` of the controller/orchestrator/node, and modify the `controller/orchestrator/node.toml` as follows:

- controller.toml: since it uses mTLS with software keys via Tonic, set all endpoints to https
- orchestrator.toml: use https toward Controller and http toward Node. Set `node_register_url` to http:
    - domain_register_url = "https://..."
    - orchestrator_url = "https://..."
    - orchestrator_url_announced = "https://..."
    - node_register_url = "http://..."
- node.toml: the following endpoints need to be http to provide support for the custom TPM signer:
    - invocation_url = "http://..."
    - invocation_url_announced = "http://..."
    - node_register_url = "http://..."

Note that the certificates here provided here should NOT be used in production.

## Generating certificates for Controller and Orchestrator
While the Node is expected to use the certificate obtained with the Registered Authentication process, you will need to generate your own CA and certificates for the Orchestrator and Controller. For this you can use the included [provision tool](https://github.com/edgeless-project/Registered-Authentication/tree/main/Provisioning) in Registered Authentication. For example, to generate a test certificate for the orchestrator, build (requires golang) and run the following command:
```
go build provision
./provision -generateCert 127.0.0.1 www.example.com 127.0.0.1 orchestrator
```
Note that you will need to keep the CA certificate and corresponding key secure (preferably in a device with no internet access), and that you will need to distribute this CA certificate to any other devices that you want to communicate with so that they trust the certificate.

## Loading private keys onto the TPM
While mTLS is intended to be used with devices that contain a TPM and that have obtained their certificate following the [Registered Authentication](https://github.com/edgeless-project/Registered-Authentication) process, you can also generate or import your already generated keys onto the TPM to provide an extra layer of security. For this, you will need to use the `tpm2_tools` package and run the following commands:
```
tpm2_createprimary -Grsa2048:aes128cfb -C o -c parent.ctx
tpm2_import -C parent.ctx -G <KEY_TYPE:rsa/ecc> -i <YOUR_PRIV_KEY> -u client.pub -r client.priv
tpm2_load -C parent.ctx -u client.pub -r client.priv -c client.ctx
tpm2_evictcontrol -C o -c client.ctx 0x81010002
```

After this commands, your private key will be stored inside of the TPM and can be used for the mTLS.