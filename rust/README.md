# Rust

* `api-tonic`: API server that publishes to kafka using `tonic`

Build. `rustix` needs nightly.

```
cargo +nightly build
```

Run

```
RUST_LOG=info cargo +nightly run
```

Send message to API

```
grpcurl -plaintext \
  -d '{"name": "Bob"}' \
  localhost:50051 \
  hello.HelloApi/SayHello
```