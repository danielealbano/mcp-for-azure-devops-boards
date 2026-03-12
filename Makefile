.PHONY: all build release run check test lint fmt clean help test-e2e-build test-e2e

# Default target
all: fmt lint test build

# Build in debug mode
build:
	cargo build

# Build in release mode
release:
	cargo build --release

# Run the application (debug mode)
run:
	cargo run

# Check if the code compiles
check:
	cargo check

# Run tests
test:
	cargo test --features test-support

# Run clippy for linting
lint:
	cargo clippy --features test-support -- -D warnings

# Format code
fmt:
	cargo fmt

# Clean build artifacts
clean:
	cargo clean

# Build Docker images for E2E tests
test-e2e-build:
	docker build -t mcp-test-claude-code tests/docker/claude-code/
	docker build -t mcp-test-cursor tests/docker/cursor/
	docker build -t mcp-test-gemini-cli tests/docker/gemini-cli/

# Run E2E testcontainers tests (requires Docker)
test-e2e: test-e2e-build
	cargo test --features test-support,e2e-tests

# Show help
help:
	@echo "Available targets:"
	@echo "  build          - Build in debug mode"
	@echo "  release        - Build in release mode"
	@echo "  run            - Run the application (debug mode)"
	@echo "  check          - Check if the code compiles"
	@echo "  test           - Run tests"
	@echo "  lint           - Run clippy for linting"
	@echo "  fmt            - Format code"
	@echo "  clean          - Clean build artifacts"
	@echo "  test-e2e-build - Build Docker images for E2E tests"
	@echo "  test-e2e       - Run E2E testcontainers tests (requires Docker)"
	@echo "  all            - Run fmt, lint, test, and build"
