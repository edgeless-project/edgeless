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

    let containers = match docker.get_containers(false) {
        Ok(containers) => containers,
        Err(e) => {
            panic!("{}", e);
        }
    };

    println!("the current containers are running:");
    for container in containers {
        println!("{}", container.Image);
    }

    let name = uuid::Uuid::new_v4();

    match docker.create_container(
        name.to_string(),
        rs_docker::container::ContainerCreate {
            Image: "hello-world".to_string(),
            Labels: None,
            ExposedPorts: None,
            HostConfig: None,
        },
    ) {
        Ok(val) => println!("{}", val),
        Err(_e) => {
            println!("could not create the container, this likely means that the hello-world image is not available locally\nplease run: docker image pull hello-world");
            return;
        }
    };

    match docker.start_container(&name.to_string()) {
        Ok(val) => println!("{}", val),
        Err(e) => {
            panic!("{}", e);
        }
    };

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
