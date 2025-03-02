IMAGE_NODE=ghcr.io/edgeless-project/edgeless_node:v1.0.0
IMAGE_ORC=ghcr.io/edgeless-project/edgeless_orc:v1.0.0
IMAGE_CON=ghcr.io/edgeless-project/edgeless_con:v1.0.0

.PHONY: all pull build run

all: pull build run

pull:
	docker pull $(IMAGE_NODE)
	docker pull $(IMAGE_ORC)
	docker pull $(IMAGE_CON)

run:
	docker run -d --name edgeless_node $(IMAGE_NODE)
	docker run -d --name edgeless_orc $(IMAGE_ORC)
	docker run -d --name edgeless_con $(IMAGE_CON)

run-bck:
	docker compose -f docker-compose-bck.yml down
	docker compose -f docker-compose-bck.yml build
	docker compose -f docker-compose-bck.yml up


restart:
	docker compose down
	docker compose build
	docker compose up

