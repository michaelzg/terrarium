#!/bin/bash

PROTO_DIR="../api-tonic/proto"
OUT_DIR="./proto"

# Create output directory if it doesn't exist
mkdir -p ${OUT_DIR}

# Generate proto descriptor file
protoc \
    -I ${PROTO_DIR} \
    --include_imports \
    --include_source_info \
    --descriptor_set_out=${OUT_DIR}/proto.pb \
    ${PROTO_DIR}/hello.proto
