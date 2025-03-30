.PHONY: infra dev api consumer ui clean stop stop-infra stop-services logs logs-api logs-consumer logs-ui setup

# Setup dependencies
setup:
	@echo "Installing UI dependencies..."
	cd ui && npm install
	@echo "Generating proto files..."
	cd ui && ./proto-gen.sh

# Run infrastructure (kafka, postgres, envoy)
infra: stop-infra
	docker compose -f local/docker-compose.yaml up -d
	@echo "Waiting for Kafka to be ready..."
	@sleep 10

# Run both API and consumer for development
dev: stop setup infra
	@echo "Starting services..."
	@mkdir -p logs
	@echo "Stopping any existing processes..."
	@lsof -ti:50051 | xargs kill -9 2>/dev/null || true
	@echo "Starting API..."
	@cd api-tonic && RUST_LOG=info API_CONFIG="$$(cat config.json)" cargo run > ../logs/api.log 2>&1 & echo "API server started (PID: $$!)"
	@sleep 2
	@echo "Starting consumer..."
	@cd consumer && RUST_LOG=info CONSUMER_CONFIG="$$(cat config.json)" cargo run > ../logs/consumer.log 2>&1 & echo "Consumer started (PID: $$!)"
	@echo "Starting UI..."
	@cd ui && npx vite > ../logs/ui.log 2>&1 & echo "UI started (PID: $$!)"
	@echo "All services started. Use 'make logs' to view logs."
	@echo "UI will be available at: http://localhost:5173"
	@echo "API (via Envoy) will be available at: http://localhost:8080"

# Run API server only
api: stop-services
	@mkdir -p logs
	@lsof -ti:50051 | xargs kill -9 2>/dev/null || true
	cd api-tonic && RUST_LOG=info API_CONFIG="$$(cat config.json)" cargo run > ../logs/api.log 2>&1

# Run consumer only
consumer: stop-services
	@mkdir -p logs
	cd consumer && RUST_LOG=info CONSUMER_CONFIG="$$(cat config.json)" cargo run > ../logs/consumer.log 2>&1

# Run UI only
ui: stop-services setup
	@mkdir -p logs
	cd ui && npx vite > ../logs/ui.log 2>&1

# View all logs
logs:
	@echo "Viewing all logs (Ctrl+C to exit)..."
	@tail -f logs/*.log

# View API logs
logs-api:
	@echo "Viewing API logs (Ctrl+C to exit)..."
	@tail -f logs/api.log

# View consumer logs
logs-consumer:
	@echo "Viewing consumer logs (Ctrl+C to exit)..."
	@tail -f logs/consumer.log

# View UI logs
logs-ui:
	@echo "Viewing UI logs (Ctrl+C to exit)..."
	@tail -f logs/ui.log

# Stop all services and infrastructure
stop: stop-services stop-infra
	@echo "Waiting for ports to be released..."
	@sleep 3

# Stop infrastructure
stop-infra:
	docker compose -f local/docker-compose.yaml down
	@echo "Waiting for containers to stop..."
	@sleep 3

# Stop Rust services and UI
stop-services:
	@echo "Stopping API, consumer, and UI..."
	@lsof -ti:50051 | xargs kill -9 2>/dev/null || true
	@pkill -f "cargo run" || true
	@pkill -f "vite" || true
	@sleep 2

# Clean all build artifacts
clean: stop
	cd api-tonic && cargo clean
	cd consumer && cargo clean
	cd ui && rm -rf node_modules dist src/generated
	rm -rf logs
