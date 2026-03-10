# Cimmeria Server Emulator — Cross-Platform Build & Run
# Usage: make all       (full pipeline)
#        make build     (compile only)
#        make start     (launch servers)
#        make stop      (shut down everything)

SHELL := /bin/bash
.DEFAULT_GOAL := help

SCRIPTS := bootstrap/scripts
OS := $(shell uname -s)

# --- Meta targets ---

.PHONY: all help clean

all: deps build db-init runtime start  ## Full pipeline: deps -> build -> db -> start

help:  ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

clean:  ## Remove build directory
	rm -rf build

# --- Dependencies ---

.PHONY: deps

deps:  ## Install platform dependencies (Homebrew or apt)
ifeq ($(OS),Darwin)
	@bash $(SCRIPTS)/deps-macos.sh
else ifeq ($(OS),Linux)
	@bash $(SCRIPTS)/deps-linux.sh
else
	@echo "Unsupported OS for 'make deps'. On Windows, use: pwsh setup.ps1"
endif

# --- Build ---

.PHONY: configure build

configure:  ## Run CMake configure
	@bash $(SCRIPTS)/build.sh configure

build: configure  ## Build all targets
	@bash $(SCRIPTS)/build.sh build

# --- Docker ---

.PHONY: docker-up docker-down

docker-up:  ## Start Docker services (PostgreSQL)
	@bash $(SCRIPTS)/docker.sh up

docker-down:  ## Stop Docker services
	@bash $(SCRIPTS)/docker.sh down

# --- Database ---

.PHONY: db-init db-reset

db-init: docker-up  ## Initialize database (load schemas if needed)
	@bash $(SCRIPTS)/database.sh init

db-reset: docker-up  ## Drop and reload database schemas
	@bash $(SCRIPTS)/database.sh reset

# --- Runtime ---

.PHONY: runtime

runtime:  ## Verify runtime library resolution
	@bash $(SCRIPTS)/runtime.sh

# --- Server ---

.PHONY: start stop

start:  ## Start all game servers
	@bash $(SCRIPTS)/server.sh start

stop:  ## Stop all game servers and Docker
	@bash $(SCRIPTS)/server.sh stop
	@bash $(SCRIPTS)/docker.sh down
