PREFIX ?= $(HOME)/.local

.PHONY: build install uninstall test check clean

build:
	cargo build --release

install: build
	install -Dm755 target/release/shanshui $(PREFIX)/bin/shanshui
	install -Dm755 scripts/omarchy-launch-screensaver $(PREFIX)/bin/omarchy-launch-screensaver
	@echo "installed to $(PREFIX)/bin — the next idle timeout uses it"

uninstall:
	rm -f $(PREFIX)/bin/shanshui $(PREFIX)/bin/omarchy-launch-screensaver

test:
	cargo test --release

check:
	cargo clippy --release -- -D warnings

clean:
	cargo clean
