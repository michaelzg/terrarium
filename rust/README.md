# Rust

* `api-tonic`: API server that publishes to kafka using `tonic`

See root README for build and run commands.

Send message to API

```
grpcurl -plaintext \
  -d '{"name": "Bob"}' \
  localhost:50051 \
  hello.HelloApi/SayHello
```