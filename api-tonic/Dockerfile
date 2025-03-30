FROM --platform=linux/amd64 rust:1.76-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/api-tonic

# Install nightly and set as default
RUN rustup default nightly

# Copy only necessary files
COPY Cargo.toml Cargo.lock ./
COPY src ./sr
COPY proto ./proto
COPY build.rs ./
COPY config.json ./

RUN cargo build

FROM --platform=linux/amd64 debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/api-tonic/target/release/helloworld-server /usr/local/bin/api-tonic
COPY --from=builder /usr/src/api-tonic/config.json /usr/local/bin/

EXPOSE 50051
CMD ["api-tonic"]
