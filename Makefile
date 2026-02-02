.PHONY: build install uninstall clean test check-rust fmt lint release help

BINARY_NAME := feynman
INSTALL_PATH := /usr/local/bin
CARGO := cargo

# Minimum Rust version
MIN_RUST_VERSION := 1.70.0

help: ## Show this help
	@echo "Feynman - Stochastic Teacher CLI"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

check-rust: ## Check if Rust is installed with minimum version
	@command -v rustc >/dev/null 2>&1 || { \
		echo "Error: Rust is not installed."; \
		echo "Install it with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; \
		exit 1; \
	}
	@RUST_VERSION=$$(rustc --version | cut -d' ' -f2); \
	if [ "$$(printf '%s\n' "$(MIN_RUST_VERSION)" "$$RUST_VERSION" | sort -V | head -n1)" != "$(MIN_RUST_VERSION)" ]; then \
		echo "Error: Rust $(MIN_RUST_VERSION) or higher required. Found: $$RUST_VERSION"; \
		exit 1; \
	fi
	@echo "Rust version OK: $$(rustc --version)"

build: check-rust ## Build the binary (debug mode)
	$(CARGO) build
	@echo "Built: target/debug/$(BINARY_NAME)"

release: check-rust ## Build the binary (release mode, optimized)
	$(CARGO) build --release
	@echo "Built: target/release/$(BINARY_NAME)"

install: release ## Build and install to /usr/local/bin
	@echo "Installing $(BINARY_NAME) to $(INSTALL_PATH)..."
	@sudo cp target/release/$(BINARY_NAME) $(INSTALL_PATH)/$(BINARY_NAME)
	@sudo chmod +x $(INSTALL_PATH)/$(BINARY_NAME)
	@echo "Installed successfully!"
	@echo ""
	@echo "Initialize the database with:"
	@echo "  $(BINARY_NAME) init"

uninstall: ## Remove from /usr/local/bin
	@echo "Removing $(BINARY_NAME) from $(INSTALL_PATH)..."
	@sudo rm -f $(INSTALL_PATH)/$(BINARY_NAME)
	@echo "Uninstalled successfully!"

clean: ## Clean build artifacts
	$(CARGO) clean
	@echo "Cleaned."

test: check-rust ## Run tests
	$(CARGO) test

fmt: ## Format code
	$(CARGO) fmt

lint: ## Run clippy lints
	$(CARGO) clippy -- -D warnings

check: fmt lint test ## Run all checks (format, lint, test)
	@echo "All checks passed!"

# Development helpers
dev: build ## Build and run with sample commands
	@echo "Running development build..."
	./target/debug/$(BINARY_NAME) --help

init-db: build ## Initialize development database
	./target/debug/$(BINARY_NAME) init
	@echo "Development database initialized."
