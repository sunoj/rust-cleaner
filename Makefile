# Build and install Rust Cleaner as a macOS .app bundle.
APP_NAME := Rust Cleaner
BUNDLE := $(APP_NAME).app
BINARY := rust-cleaner
INSTALL_DIR := /Applications
PLIST := com.wd40.rust-cleaner.plist
LAUNCH_AGENTS_DIR := $(HOME)/Library/LaunchAgents

.PHONY: build bundle install uninstall autostart no-autostart clean

build:
	cargo build --release

bundle: build
	rm -rf "$(BUNDLE)"
	mkdir -p "$(BUNDLE)/Contents/MacOS" "$(BUNDLE)/Contents/Resources"
	cp target/release/$(BINARY) "$(BUNDLE)/Contents/MacOS/"
	cp Info.plist "$(BUNDLE)/Contents/"
	cp AppIcon.icns "$(BUNDLE)/Contents/Resources/"
	@echo "Built $(BUNDLE)"

install: bundle
	rm -rf "$(INSTALL_DIR)/$(BUNDLE)"
	cp -R "$(BUNDLE)" "$(INSTALL_DIR)/"
	@echo "Installed to $(INSTALL_DIR)/$(BUNDLE)"

uninstall: no-autostart
	rm -rf "$(INSTALL_DIR)/$(BUNDLE)"
	@echo "Uninstalled $(APP_NAME)"

autostart:
	mkdir -p "$(LAUNCH_AGENTS_DIR)"
	cp $(PLIST) "$(LAUNCH_AGENTS_DIR)/$(PLIST)"
	@echo "Auto-start enabled (takes effect next login)"

no-autostart:
	rm -f "$(LAUNCH_AGENTS_DIR)/$(PLIST)"
	-launchctl bootout gui/$$(id -u) $(PLIST) 2>/dev/null
	@echo "Auto-start disabled"

clean:
	cargo clean
	rm -rf "$(BUNDLE)"
