VERSION ?= $(shell cargo metadata --no-deps --format-version 1 2>/dev/null | jq -r '.packages[0].version' || sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
TARGET ?= $(shell rustc -Vv | sed -n 's/^host: //p')
DIST_DIR ?= dist
PKG_NAME = hx-v$(VERSION)-$(TARGET)
PKG_DIR = $(DIST_DIR)/$(PKG_NAME)

.PHONY: all build check test lint fmt fmt-check bench clean dist package checksums install uninstall cross-check cross-build changelog bump release help

all: build

help:
	@echo "hx makefile targets:"
	@echo "  build         - Build release binaries for host target"
	@echo "  check         - Run cargo check across all targets and features"
	@echo "  test          - Run test suite across all targets and features"
	@echo "  lint          - Run cargo clippy with -D warnings"
	@echo "  fmt           - Format code with cargo fmt"
	@echo "  fmt-check     - Check code formatting"
	@echo "  bench         - Compile benchmarks without running"
	@echo "  dist          - Build release and create tar.gz package + sha256 checksum"
	@echo "  checksums     - Regenerate SHA256SUMS for all archives in dist/"
	@echo "  cross-check   - Run cross check for musl and aarch64 targets"
	@echo "  cross-build   - Build musl and aarch64 release binaries using cross"
	@echo "  install       - Install hx into ~/.local/bin"
	@echo "  uninstall     - Remove hx from ~/.local/bin"
	@echo "  changelog     - Generate CHANGELOG.md via git-cliff"
	@echo "  release       - Bump version according to semver and tag release"
	@echo "  clean         - Clean build target directory and dist/"

build:
	@cargo build --release

check:
	@cargo check --all-targets --all-features

test:
	@cargo test --all-targets --all-features

lint:
	@cargo clippy --all-targets --all-features -- -D warnings

fmt:
	@cargo fmt --all

fmt-check:
	@cargo fmt --all -- --check

bench:
	@cargo bench --no-run

clean:
	@cargo clean
	@rm -rf $(DIST_DIR)

package: dist

dist: build
	@mkdir -p $(PKG_DIR)/config/tmux
	@cp target/release/hx $(PKG_DIR)/
	@cp README.md LICENSE $(PKG_DIR)/
	@cp config/tmux/hx.conf $(PKG_DIR)/config/tmux/
	@cp install.sh $(PKG_DIR)/
	@chmod +x $(PKG_DIR)/hx $(PKG_DIR)/install.sh
	@tar -czf $(DIST_DIR)/$(PKG_NAME).tar.gz -C $(DIST_DIR) $(PKG_NAME)
	@rm -rf $(PKG_DIR)
	@cd $(DIST_DIR) && (sha256sum $(PKG_NAME).tar.gz 2>/dev/null || shasum -a 256 $(PKG_NAME).tar.gz) > $(PKG_NAME).tar.gz.sha256
	@$(MAKE) checksums
	@echo "Created package: $(DIST_DIR)/$(PKG_NAME).tar.gz"

checksums:
	@if [ -d "$(DIST_DIR)" ]; then \
		cd $(DIST_DIR) && \
		(sha256sum *.tar.gz 2>/dev/null || shasum -a 256 *.tar.gz) > SHA256SUMS 2>/dev/null || true; \
		echo "Generated $(DIST_DIR)/SHA256SUMS"; \
	fi

cross-check:
	@which cross >/dev/null 2>&1 || (echo "Error: 'cross' command not found. Install via: cargo install cross" && exit 1)
	@echo "==> Running cross check for x86_64-unknown-linux-musl..."
	@cross check --target x86_64-unknown-linux-musl
	@echo "==> Running cross check for aarch64-unknown-linux-gnu..."
	@cross check --target aarch64-unknown-linux-gnu

cross-build:
	@which cross >/dev/null 2>&1 || (echo "Error: 'cross' command not found. Install via: cargo install cross" && exit 1)
	@echo "==> Building x86_64-unknown-linux-musl..."
	@cross build --release --target x86_64-unknown-linux-musl
	@echo "==> Building aarch64-unknown-linux-gnu..."
	@cross build --release --target aarch64-unknown-linux-gnu

install: build
	@mkdir -p $(HOME)/.local/bin
	@rm -f $(HOME)/.local/bin/hx $(HOME)/.local/bin/ax $(HOME)/.local/bin/agymux $(HOME)/.local/bin/agemux
	@cp target/release/hx $(HOME)/.local/bin/hx
	@echo "Installed hx in $(HOME)/.local/bin"

uninstall:
	@rm -f $(HOME)/.local/bin/hx $(HOME)/.local/bin/ax $(HOME)/.local/bin/agymux $(HOME)/.local/bin/agemux
	@echo "Removed hx from $(HOME)/.local/bin"

changelog:
	@which git-cliff >/dev/null 2>&1 || (echo "Error: 'git-cliff' command not found. Install via: cargo install git-cliff" && exit 1)
	@git cliff -o CHANGELOG.md
	@echo "Updated CHANGELOG.md"

bump:
	@echo "Current version: $(VERSION)"
	@read -p "Enter new version (e.g. 0.8.6): " new_version; \
	sed -i.bak "s/^version = \".*\"/version = \"$$new_version\"/" Cargo.toml && rm -f Cargo.toml.bak; \
	cargo check >/dev/null 2>&1; \
	echo "Bumped version to $$new_version"

release: test
	@if [ -n "$$(git status --porcelain)" ]; then \
		echo "Error: Working directory has uncommitted changes. Commit or stash them first." && exit 1; \
	fi
	@echo "Creating release tag v$(VERSION)..."
	@git tag -a "v$(VERSION)" -m "Release v$(VERSION)"
	@echo "Tag v$(VERSION) created. Push with: git push origin v$(VERSION)"
