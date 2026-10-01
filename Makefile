PREFIX ?= $(HOME)/.local
OVERRIDE_BIN ?= $(HOME)/.local/overrides/bin

.PHONY: build install uninstall test check clean

build:
	cargo build --release

install: build
	install -Dm755 target/release/shanshui $(PREFIX)/bin/shanshui
	install -Dm755 scripts/omarchy-launch-screensaver $(OVERRIDE_BIN)/omarchy-launch-screensaver
	@echo "installed; try it now with: $(OVERRIDE_BIN)/omarchy-launch-screensaver force"
	@case ":$$PATH:" in *":$(OVERRIDE_BIN):"*) ;; *) echo "NOTE: $(OVERRIDE_BIN) is not on PATH yet - see README, Install"; esac

uninstall:
	rm -f $(PREFIX)/bin/shanshui $(OVERRIDE_BIN)/omarchy-launch-screensaver

test:
	cargo test --release

check:
	cargo clippy --release -- -D warnings

clean:
	cargo clean
