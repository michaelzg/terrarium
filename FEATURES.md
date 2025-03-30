# Feature Development Plan

## Current State
The project currently implements a gRPC API server that publishes messages to Kafka:
- gRPC endpoint `/hello.HelloApi/SayHello` accepting name parameter hosted by api-tonic.
- Messages published to "default-topic" in Kafka
- Consumer reads from the "default-topic" in Kafka and writes to PostgreSQL database.
- Local Kafka and PostgeSQL setup via docker-compose

## Immediate Development Goals

### Step 1. UI to display and write the written mesages in api-tonic

The UI will be implemented as a modern React application that provides an intuitive interface for interacting with the gRPC API.

#### Technical Architecture
- React + TypeScript frontend using Vite
- gRPC-Web for API communication
- Material-UI (MUI) for component styling
- vis-timeline for message visualization

#### Components

1. HelloForm
- Text input for name parameter with default value 'world'
- Submit button to trigger SayHello RPC
- Real-time feedback on request status
- Loading state indicators

2. MessageTimeline
- Interactive timeline visualization of messages
- Features:
  * Dynamic clustering of nearby data points
  * Zoom controls for timeline navigation
  * Hover cards displaying full message metadata
  * Auto-refresh capability
  * Configurable message limit

#### Setup Instructions
1. Install dependencies
2. Configure gRPC-Web proxy
3. Start the development server

#### Usage
1. SayHello:
   - Enter name (or use default 'world')
   - Click submit to send message
   - View confirmation of message sent

2. View Messages:
   - Browse timeline of messages
   - Use zoom controls to navigate
   - Hover over points to view details
   - Configure refresh rate and message limit

### Step 2. Simulator to make write requests to the API. Simulation should have a validation mode to read requests to ensure the data written is there.

### Step 3. Grafana monitoring dashboard with Influx and prometheus setup to monitor application health.
