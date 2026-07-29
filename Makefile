# Build and install WD-40 as a macOS .app bundle plus the wd40 CLI.
APP_NAME := WD-40
BUNDLE := dist/$(APP_NAME).app
INSTALL_DIR := /Applications
CARGO_OUT := $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)/release

.PHONY: build test bundle migrate-legacy install uninstall cli release clean

build:
	cargo build --release

test:
	cargo test

# Produces dist/WD-40.app with Sparkle embedded and everything code-signed.
bundle:
	./scripts/bundle.sh

# One-shot cleanup of the pre-0.5.0 name. Without it an upgrading user keeps
# the old bundle and its login job, and two menu bar apps start at login.
# Delete this target and its use once no 0.4.x install is left in the wild.
migrate-legacy:
	-launchctl bootout gui/$$(id -u)/com.wd40.rust-cleaner 2>/dev/null
	rm -f "$(HOME)/Library/LaunchAgents/com.wd40.rust-cleaner.plist"
	rm -rf "$(INSTALL_DIR)/Rust Cleaner.app"

install: bundle migrate-legacy
	rm -rf "$(INSTALL_DIR)/$(APP_NAME).app"
	cp -R "$(BUNDLE)" "$(INSTALL_DIR)/"
	@echo "Installed $(INSTALL_DIR)/$(APP_NAME).app — enable Launch at Login from the Settings submenu"

uninstall:
	rm -rf "$(INSTALL_DIR)/$(APP_NAME).app"
	rm -f "$(HOME)/Library/LaunchAgents/com.wd40.app.plist"
	-launchctl bootout gui/$$(id -u)/com.wd40.app 2>/dev/null
	@echo "Uninstalled $(APP_NAME)"

# Sandbox provenance blocks execution until the copy is re-signed.
cli: build
	cp "$(CARGO_OUT)/wd40" "$(HOME)/.cargo/bin/wd40"
	codesign --force --sign - "$(HOME)/.cargo/bin/wd40"
	@echo "Installed wd40 CLI to $(HOME)/.cargo/bin"

# Publishes to the Sparkle feed: make release VERSION=0.5.0 NOTES="..."
release:
	@test -n "$(VERSION)" || (echo "usage: make release VERSION=x.y.z [NOTES=...]" && exit 1)
	./scripts/release.sh "$(VERSION)" "$(NOTES)"

clean:
	cargo clean
	rm -rf dist
