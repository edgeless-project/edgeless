// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[test]
fn test_docker_basic() {
    let path = "/var/run/docker.sock";
    if !std::path::Path::new(path).exists() {
        println!(
            "the path to Docker ({}) does not exist, this likely means that Docker is not installed: bailing out ot the test",
            path
        );
        return;
    }

    let mut docker = match rs_docker::Docker::connect(format!("unix://{}", path).as_str()) {
        Ok(docker) => docker,
        Err(e) => {
            println!("could not connect to Docker, bailing out of the test: {}", e);
            return;
        }
    };

    let images = match docker.get_images(false) {
        Ok(images) => images,
        Err(e) => {
            println!("could not list images, bailing out of the test: {}", e);
            return;
        }
    };

    println!("the current images are available locally:");
    for image in images {
        if !image.RepoTags.is_empty() {
            println!("\t{}", image.RepoTags.first().unwrap());
        }
    }

    let containers = match docker.get_containers(false) {
        Ok(containers) => containers,
        Err(e) => {
            panic!("{}", e);
        }
    };

    println!("the current containers are running:");
    for container in containers {
        println!("\t{}", container.Image);
    }

    let name = uuid::Uuid::new_v4();
    let image_name = "edgeless_function".to_string();

    match docker.create_container(
        name.to_string(),
        rs_docker::container::ContainerCreate {
            Image: image_name.clone(),
            Labels: None,
            ExposedPorts: None,
            HostConfig: Some(rs_docker::container::HostConfigCreate {
                NetworkMode: None,
                PublishAllPorts: Some(true),
                PortBindings: None,
            }),
        },
    ) {
        Ok(val) => println!("{}", val),
        Err(_e) => {
            println!(
                "could not create the container, this likely means that the image is not available locally\nplease run: docker image pull {}",
                image_name
            );
            return;
        }
    };

    match docker.start_container(&name.to_string()) {
        Ok(val) => println!("{}", val),
        Err(e) => {
            println!("error when starting the container, this likely means Docker is misconfigured: {}", e);
            return;
        }
    };

    let containers = match docker.get_containers(false) {
        Ok(containers) => containers,
        Err(e) => {
            panic!("{}", e);
        }
    };

    let container = containers
        .iter()
        .find(|x| {
            if let Some(name_found) = x.Names.first() {
                if *name_found == format!("/{}", name) {
                    return true;
                }
            }
            false
        })
        .unwrap();
    let port = container
        .Ports
        .iter()
        .find(|x| {
            if let Some(ip) = &x.IP {
                if ip == "0.0.0.0" {
                    return true;
                }
            }
            false
        })
        .unwrap();
    println!(
        "port: {}:{}->{}/{}",
        port.IP.as_ref().unwrap_or(&"".to_string()),
        port.PrivatePort,
        port.PublicPort.unwrap_or_default(),
        port.Type
    );

    match docker.stop_container(&name.to_string()) {
        Ok(val) => println!("{}", val),
        Err(e) => {
            panic!("{}", e);
        }
    };

    match docker.delete_container(&name.to_string()) {
        Ok(val) => println!("{}", val),
        Err(e) => {
            panic!("{}", e);
        }
    };
}

#[test]
fn test_docker_basic_with_utils() {
    let mut docker = match crate::container_runner::docker_utils::Docker::connect() {
        Ok(docker) => docker,
        Err(err) => {
            println!("could not connect to Docker, which may fine: {}", err);
            return;
        }
    };

    let image_name = "edgeless_function".to_string();

    let (id, port) = match crate::container_runner::docker_utils::Docker::start(&mut docker, image_name) {
        Ok((id, port)) => (id, port),
        Err(err) => {
            println!("could not create container, which may be fine: {}", err);
            return;
        }
    };
    println!("container ID: {}, port: {}", id, port);
    crate::container_runner::docker_utils::Docker::stop(&mut docker, id).expect("we should be able to stop a container that started flawlessly");
}
