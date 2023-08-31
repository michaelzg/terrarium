RUST_DIR = rust/api-tonic

.PHONY: build run

build:
	cargo +nightly build --manifest-path=$(RUST_DIR)/Cargo.toml

run:
	RUST_LOG=info cargo +nightly run --manifest-path=$(RUST_DIR)/Cargo.toml

clean:
	RUST_LOG=info cargo clean --manifest-path=$(RUST_DIR)/Cargo.toml
