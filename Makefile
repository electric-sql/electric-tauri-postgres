.PHONY: all build test dev

all: build test

test:
	pnpm test

dev:
	pnpm run tauri dev
