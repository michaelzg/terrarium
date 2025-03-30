RUST_DIR = api

.PHONY: build run clean kafka consumer deps deps-up deps-down

build:
	cargo build --manifest-path=$(RUST_DIR)/Cargo.toml

run:
	RUST_LOG=info cargo run --manifest-path=$(RUST_DIR)/Cargo.toml

clean:
	RUST_LOG=info cargo clean --manifest-path=$(RUST_DIR)/Cargo.toml

# Start the dependencies in detached mode
deps-up:
	docker-compose -f local/docker-compose.yaml up -d

# Stop and remove containers, networks, and volumes
deps-down:
	docker-compose -f local/docker-compose.yaml down -v

# Ensure a clean environment by stopping any existing containers,
# removing volumes, and starting fresh containers
deps: deps-down
	docker volume rm -f terrarium_postgres-data || true
	docker-compose -f local/docker-compose.yaml up -d
	@echo "Dependencies are now running in the background."
	@echo "Use 'make deps-down' to stop and clean up when finished."

# Alias for backward compatibility
kafka: deps

consumer:
	cd consumer && RUST_LOG=info CONSUMER_CONFIG="$$(cat config.json)" cargo run
