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

Sent message from `api-tonic/`

```
grpcurl -plaintext \
  -d '{"name": "Tonic"}' \
  localhost:50051 \
  hello.HelloApi/SayHello
```