SHELL := /bin/bash

# Default docs port; override with: make docs PORT=5180
PORT ?= 5173

.PHONY: hooks docs docs-build docs-ci
hooks:
	@git config core.hooksPath .githooks
	@chmod +x .githooks/* 2>/dev/null || true
	@echo "[hooks] Installed git hooks from .githooks (core.hooksPath)"

# Start VitePress dev server and open the browser automatically.
docs:
	@echo "[docs] Ensuring deps..."
	@npm install --silent
	@echo "[docs] Starting VitePress on http://localhost:$(PORT) ..."
	# Start server in background and record PID
	@ (npm run --silent docs:dev -- --port $(PORT) &) ; \
	  server_pid=$$! ; \
	  echo "[docs] Waiting for server to become ready..." ; \
	  for i in {1..80}; do \
	    if curl -sSf "http://localhost:$(PORT)/" >/dev/null ; then \
	      echo "[docs] Server is up at http://localhost:$(PORT)/" ; \
	      scripts/docs-open.sh "http://localhost:$(PORT)/" ; \
	      wait $$server_pid ; \
	      exit 0 ; \
	    fi ; \
	    sleep 0.25 ; \
	  done ; \
	  echo "[docs] Timed out waiting for VitePress." ; \
	  exit 1

# Build static site
docs-build:
	@npm run --silent docs:build

# Build docs without installing dependencies (for CI caches)
docs-ci:
	@echo "[docs] CI build (no npm install)" 
	@npm run --silent docs:build
