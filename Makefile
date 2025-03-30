RUST_DIR = api

.PHONY: build run clean kafka consumer

build:
	cargo build --manifest-path=$(RUST_DIR)/Cargo.toml

run:
	RUST_LOG=info cargo run --manifest-path=$(RUST_DIR)/Cargo.toml

clean:
	RUST_LOG=info cargo clean --manifest-path=$(RUST_DIR)/Cargo.toml

deps:
	docker-compose -f local/docker-compose.yaml up

consumer:
	cd consumer && RUST_LOG=info CONSUMER_CONFIG="$$(cat config.json)" cargo run
