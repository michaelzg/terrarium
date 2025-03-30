#!/bin/bash

PROTO_DIR="../api-tonic/proto"
OUT_DIR="./src/generated"

# Create output directory if it doesn't exist
mkdir -p ${OUT_DIR}

# Generate JavaScript code
protoc \
    -I=${PROTO_DIR} \
    --js_out=import_style=es6,binary:${OUT_DIR} \
    --grpc-web_out=import_style=typescript,mode=grpcwebtext:${OUT_DIR} \
    ${PROTO_DIR}/hello.proto

# Convert CommonJS to ES modules
sed -i '' 's/const grpc = {};/import * as grpcWeb from "grpc-web";/g' ${OUT_DIR}/hello_grpc_web_pb.js
sed -i '' 's/grpc.web = require(.*/const grpc = { web: grpcWeb };/g' ${OUT_DIR}/hello_grpc_web_pb.js
sed -i '' 's/const proto = {};/import * as hello_pb from ".\/hello_pb.js";/g' ${OUT_DIR}/hello_grpc_web_pb.js
sed -i '' 's/proto.hello = require(.*/const proto = { hello: hello_pb };/g' ${OUT_DIR}/hello_grpc_web_pb.js
sed -i '' 's/module.exports = proto.hello;/export { HelloApiClient, HelloApiPromiseClient };/g' ${OUT_DIR}/hello_grpc_web_pb.js

# Make the script executable
chmod +x ./proto-gen.sh

echo "Proto files generated in ${OUT_DIR}"
ls -la ${OUT_DIR}
