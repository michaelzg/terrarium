RUST_DIR = api-tonic

.PHONY: build run clean kafka

build:
	cargo build --manifest-path=$(RUST_DIR)/Cargo.toml

run:
	RUST_LOG=info cargo run --manifest-path=$(RUST_DIR)/Cargo.toml

clean:
	RUST_LOG=info cargo clean --manifest-path=$(RUST_DIR)/Cargo.toml

kafka:
	docker-compose -f local/docker-compose.kafka.yaml up

it:
	cargo test --manifest-path=$(RUST_DIR)/Cargo.toml --test integration_test -- --show-output
