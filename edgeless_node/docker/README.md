# Example container function for EDGELESS

## Prerequisites

[Install Docker](https://docs.docker.com/get-docker/)

## Build the image

The image can be built with the following command, to be executed from the repo
root:

```bash
docker build -t edgeless_function edgeless_node/docker/
```

Once built you can create function instances in an EDGELESS system by
specifying 

```json
"code": "container:edgeless_function:latest",
```

in the `class_specification` object of the workflow JSON.