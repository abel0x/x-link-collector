# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 abel0x <https://github.com/abel0x>
# X Link Collector -- build, test and install helpers.
# Recipes use tabs; `make` with no target prints this file's help.

BIN_NAME := x-link-receiver
CARGO    ?= cargo
UNAME    := $(shell uname -s)
# Clippy runs per target: a lint can fire on one platform and not another,
# which is exactly how a Windows-only `needless_return` reached CI once.
CI_TARGETS := x86_64-unknown-linux-gnu x86_64-pc-windows-gnu aarch64-apple-darwin
PREFIX   ?= $(HOME)/.local
BIN      := receiver/target/release/$(BIN_NAME)
UNIT_DIR := $(HOME)/.config/systemd/user
UNIT     := $(UNIT_DIR)/$(BIN_NAME).service
PLIST_ID := com.abel0x.x-link-receiver

.DEFAULT_GOAL := help
.PHONY: help build ci test test-rs test-ext run install uninstall service \
	    service-stop service-uninstall status logs package clean

help: ## show this help
	@awk 'BEGIN { FS = ":.*## " } /^[a-z][a-z-]*:.*## / { printf "  make %-18s %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

build: ## compile the receiver in release mode
	$(CARGO) build --release --manifest-path receiver/Cargo.toml

ci: ## run every check CI runs, including the cross-target ones
	$(CARGO) fmt --manifest-path receiver/Cargo.toml --check
	$(CARGO) test --manifest-path receiver/Cargo.toml
	@for t in $(CI_TARGETS); do \
	  printf 'clippy %s\n' "$$t"; \
	  $(CARGO) clippy --manifest-path receiver/Cargo.toml --target "$$t" \
	    --all-targets -- -D warnings || exit 1; \
	done
	node --test extension/test/
	python3 downloader/x-download --help >/dev/null
	python3 spliter/x-flatten --help >/dev/null
	@echo "all clear"

test: test-rs test-ext ## run every test

test-rs: ## run the receiver's unit tests
	$(CARGO) test --manifest-path receiver/Cargo.toml

test-ext: ## run the service worker's tests (needs node)
	node --test extension/test/

run: build ## run the receiver in the foreground (Ctrl-C to stop)
	$(BIN)

install: build ## copy the binary to ~/.local/bin (override PREFIX)
	install -Dm755 $(BIN) $(PREFIX)/bin/$(BIN_NAME)
	@echo "installed $(PREFIX)/bin/$(BIN_NAME)"

uninstall: ## remove the installed binary
	rm -f $(PREFIX)/bin/$(BIN_NAME)

service: install ## run the receiver at login (systemd on Linux, launchd on macOS)
ifeq ($(UNAME),Darwin)
	@mkdir -p $(HOME)/Library/LaunchAgents
	@sed 's|__BIN__|$(PREFIX)/bin/$(BIN_NAME)|' receiver/launchd/$(PLIST_ID).plist \
	  > $(HOME)/Library/LaunchAgents/$(PLIST_ID).plist
	-launchctl unload $(HOME)/Library/LaunchAgents/$(PLIST_ID).plist 2>/dev/null
	launchctl load -w $(HOME)/Library/LaunchAgents/$(PLIST_ID).plist
	@echo "loaded $(PLIST_ID)"
else
	install -Dm644 receiver/systemd/$(BIN_NAME).service $(UNIT)
	systemctl --user daemon-reload
	systemctl --user enable --now $(BIN_NAME).service
	@systemctl --user --no-pager --lines=0 status $(BIN_NAME).service || true
endif

service-stop: ## stop the service without uninstalling it
ifeq ($(UNAME),Darwin)
	launchctl unload $(HOME)/Library/LaunchAgents/$(PLIST_ID).plist
else
	systemctl --user disable --now $(BIN_NAME).service
endif

service-uninstall: ## stop the service and remove its unit file
ifeq ($(UNAME),Darwin)
	-launchctl unload $(HOME)/Library/LaunchAgents/$(PLIST_ID).plist 2>/dev/null
	rm -f $(HOME)/Library/LaunchAgents/$(PLIST_ID).plist
else
	-systemctl --user disable --now $(BIN_NAME).service
	rm -f $(UNIT)
	systemctl --user daemon-reload
endif

status: ## is the receiver up, and how many links does it hold?
	@curl -fsS http://127.0.0.1:9876/health && echo || echo "receiver is not responding"

logs: ## follow the service log
ifeq ($(UNAME),Darwin)
	tail -f /tmp/$(BIN_NAME).log
else
	journalctl --user -u $(BIN_NAME).service -f
endif

package: ## zip the extension for distribution (dist/)
	@mkdir -p dist
	cd extension && zip -qr ../dist/x-link-collector.zip . -x 'test/*'
	@echo "wrote dist/x-link-collector.zip"

downloader-setup: ## install yt-dlp + gallery-dl into downloader/.venv (no sudo)
	python3 -m venv --clear downloader/.venv
	downloader/.venv/bin/python -m pip install --quiet --upgrade pip yt-dlp gallery-dl
	@printf '  yt-dlp     %s\n' "$$(downloader/.venv/bin/python -m yt_dlp --version)"
	@printf '  gallery-dl %s\n' "$$(downloader/.venv/bin/python -m gallery_dl --version)"

downloader-update: ## upgrade yt-dlp/gallery-dl (X breaks them every few weeks)
	downloader/.venv/bin/python -m pip install --quiet --upgrade yt-dlp gallery-dl
	@printf '  yt-dlp     %s\n' "$$(downloader/.venv/bin/python -m yt_dlp --version)"

download: ## download media for every link not fetched yet
	downloader/x-download

download-watch: ## keep downloading as the extension collects new links
	downloader/x-download --watch

download-all: ## download every link incl. age-restricted ones (mirror only)
	downloader/x-download --mirror-only

download-status: ## show what has been downloaded so far
	downloader/x-download --status

flatten: ## move downloaded media into one flat folder (dry run first!)
	spliter/x-flatten

flatten-undo: ## put flattened files back into their per-handle folders
	spliter/x-flatten --undo

clean: ## remove build output
	$(CARGO) clean --manifest-path receiver/Cargo.toml
	rm -rf dist
