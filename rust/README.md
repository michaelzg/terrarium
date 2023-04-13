# Rust

* `api-tonic`: API server that publishes to kafka using `tonic`

Build. `rustix` needs nightly.

```
cargo +nightly build
```

Run

```
cargo +nightly run
```

Sent message from `api-tonic/`

```
grpcurl -plaintext -import-path ./proto -proto helloworld.proto -d '{"name": "Tonic"}' localhost:50051 helloworld.Greeter/SayHello
```