# Feature Development Plan

## Current State
The project currently implements a gRPC API server that publishes messages to Kafka:
- gRPC endpoint `/hello.HelloApi/SayHello` accepting name parameter hosted by api.
- Messages published to "default-topic" in Kafka
- Consumer reads from the "default-topic" in Kafka and writes to PostgreSQL database.
- Local Kafka and PostgeSQL setup via docker-compose

## Immediate Development Goals

### Step 1. API to read from the written mesages in PostgreSQL

### Step 2. UI to display and write the written mesages in api

### Step 3. Simulator to make write requests to the API. Simulation should have a validation mode to read requests to ensure the data written is there.

### Step 4. Grafana monitoring dashboard with Influx and prometheus setup to monitor application health.
