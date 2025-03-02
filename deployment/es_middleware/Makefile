.DEFAULT_GOAL := help

COMPOSE_FILES := docker-compose.yml
KC_COMPOSE_FILES := docker-compose-kc.yml

up:
	@echo "Starting all services..."
	@docker-compose -f $(COMPOSE_FILES) up -d

up-kc:
	@echo "Starting Keycloak services..."
	@docker-compose -f $(KC_COMPOSE_FILES) up -d

down:
	@echo "Stopping all services..."
	@docker-compose -f $(COMPOSE_FILES) down

down-kc:
	@echo "Stopping Keycloak services..."
	@docker-compose -f $(KC_COMPOSE_FILES) down

logs:
	@echo "Showing logs for all services..."
	@docker-compose -f $(COMPOSE_FILES) logs -f

logs-kc:
	@echo "Showing logs for Keycloak services..."
	@docker-compose -f $(KC_COMPOSE_FILES) logs -f

help:
	@echo "Available commands:"
	@echo "  make up       - Start all services (using docker-compose.yml)"
	@echo "  make up-kc    - Start only Keycloak services (using docker-compose-kc.yml)"
	@echo "  make down     - Stop all services (using docker-compose.yml)"
	@echo "  make down-kc  - Stop only Keycloak services (using docker-compose-kc.yml)"
	@echo "  make logs     - Show logs for all services"
	@echo "  make logs-kc  - Show logs for Keycloak services"
	@echo "  make help     - Show this help message"