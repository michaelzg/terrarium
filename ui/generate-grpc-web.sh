#!/bin/bash

PROTO_DIR="../api-tonic/proto"
OUT_DIR="./src/generated"

# Create output directory if it doesn't exist
mkdir -p ${OUT_DIR}

# Generate JavaScript and TypeScript code
protoc -I=${PROTO_DIR} \
  --js_out=import_style=commonjs:${OUT_DIR} \
  --grpc-web_out=import_style=typescript,mode=grpcwebtext:${OUT_DIR} \
  ${PROTO_DIR}/hello.proto

# Fix imports in generated files to use relative paths
sed -i '' 's/require("\.\/hello_pb/require("\.\/hello_pb/g' ${OUT_DIR}/hello_grpc_web_pb.js
sed -i '' 's/from "\.\/hello_pb/from "\.\/hello_pb/g' ${OUT_DIR}/hello_grpc_web_pb.d.ts
