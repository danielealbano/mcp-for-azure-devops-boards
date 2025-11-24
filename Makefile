.PHONY: all build release run check test lint fmt clean help

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
	cargo test

# Run clippy for linting
lint:
	cargo clippy -- -D warnings

# Format code
fmt:
	cargo fmt

# Clean build artifacts
clean:
	cargo clean

# Show help
help:
	@echo "Available targets:"
	@echo "  build   - Build in debug mode"
	@echo "  release - Build in release mode"
	@echo "  run     - Run the application (debug mode)"
	@echo "  check   - Check if the code compiles"
	@echo "  test    - Run tests"
	@echo "  lint    - Run clippy for linting"
	@echo "  fmt     - Format code"
	@echo "  clean   - Clean build artifacts"
	@echo "  all     - Run fmt, lint, test, and build"
