// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// Docker utilities.
pub struct Docker {}

impl Docker {
    /// Connect to a Docker daemon.
    pub fn connect() -> anyhow::Result<rs_docker::Docker> {
        let path = "/var/run/docker.sock";
        if !std::path::Path::new(path).exists() {
            Err(anyhow::anyhow!(
                "the path to Docker ({}) does not exist, this likely means that Docker is not installed: bailing out ot the test",
                path
            ))
        } else {
            match rs_docker::Docker::connect(format!("unix://{}", path).as_str()) {
                Ok(docker) => Ok(docker),
                Err(err) => Err(anyhow::anyhow!("could not connect to Docker, bailing out: {}", err)),
            }
        }
    }

    /// Start a container with a given image, which must be available locally.
    /// The name is automatically selected as a random UUID; the ID is returned.
    /// It is assumed that a TCP port bound to 0.0.0.0 is published, the
    /// public port number is returned, too.
    pub fn start(docker: &mut rs_docker::Docker, image_name: String) -> anyhow::Result<(String, u64)> {
        let name: String = uuid::Uuid::new_v4().to_string();

        let mut devices = vec![];

        // SecureExecutor will create trusted containers. In all these cases, image names
        // have the following pattern "edgeless-sgx-function-<language>-<function_name>"
        // Hence, if this pattern is detected, this means that we need to pass the SGX driver to the container
        // This is mandatory to utilize SGX functionalities from within the container
        // NUC devices are used for now that support SGX in the edge devices
        if image_name.contains("edgeless-sgx-function-") {
            let sgx_nuc_driver = crate::container_runner::container_devices::get_sgx_nuc_driver();
            devices.push(sgx_nuc_driver);
        }

        let id = match docker.create_container(
            name.to_string(),
            rs_docker::container::ContainerCreate {
                Image: image_name.clone(),
                Labels: None,
                ExposedPorts: None,
                HostConfig: Some(rs_docker::container::HostConfigCreate {
                    NetworkMode: None,
                    PublishAllPorts: Some(true),
                    PortBindings: None,
                    Devices: Some(devices),
                }),
            },
        ) {
            Ok(id) => id,
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "could not create the container, this likely means that the image is not available locally, please run: docker image pull {}: {}",
                    image_name,
                    err
                ));
            }
        };

        if let Err(err) = docker.start_container(&name.to_string()) {
            return Err(anyhow::anyhow!("could not start the container with image {}: {}", image_name, err));
        }

        let containers = match docker.get_containers(false) {
            Ok(containers) => containers,
            Err(err) => return Err(anyhow::anyhow!("could not list the containers: {}", err)),
        };

        let container = match containers.iter().find(|x| id == x.Id) {
            Some(container) => container,
            None => return Err(anyhow::anyhow!("could not find the newly-created container with ID {}", id)),
        };
        let public_port = match container.Ports.iter().find(|x| {
            if let Some(ip) = &x.IP
                && ip == "0.0.0.0" {
                    return true;
                }
            false
        }) {
            Some(port) => match port.PublicPort {
                Some(val) => val,
                None => {
                    return Err(anyhow::anyhow!(
                        "could not find a public port to which {} is mapped for the newly-created container with ID {}",
                        port.PrivatePort,
                        id
                    ));
                }
            },
            None => {
                return Err(anyhow::anyhow!(
                    "could not find a published port bound to 0.0.0.0 the newly-created container with ID {}",
                    id
                ));
            }
        };

        Ok((id, public_port))
    }

    /// Stop and delete the container with a given ID.
    pub fn stop(docker: &mut rs_docker::Docker, id: String) -> anyhow::Result<()> {
        if let Err(err) = docker.stop_container(&id) {
            return Err(anyhow::anyhow!("could not stop container with ID {}: {}", id, err));
        }

        if let Err(err) = docker.delete_container(&id) {
            return Err(anyhow::anyhow!("could not delete container with ID {}: {}", id, err));
        }

        Ok(())
    }
}
