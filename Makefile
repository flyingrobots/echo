SHELL := /bin/bash

# Default docs port; override with: make docs PORT=5180
PORT ?= 5173

.PHONY: hooks docs docs-build docs-ci echo-total
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
	    sleep 1 ; \
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

# Generate docs/echo-total.md rollup
echo-total:
	@chmod +x scripts/gen-echo-total.sh
	@./scripts/gen-echo-total.sh
# Benchmarks and reports
.PHONY: bench-report vendor-d3 bench-serve bench-open

vendor-d3:
	@mkdir -p docs/benchmarks/vendor
	@if [ ! -f docs/benchmarks/vendor/d3.v7.min.js ]; then \
	  echo "Downloading D3 v7 to docs/benchmarks/vendor..."; \
	  curl -fsSL https://unpkg.com/d3@7/dist/d3.min.js -o docs/benchmarks/vendor/d3.v7.min.js; \
	  echo "D3 saved to docs/benchmarks/vendor/d3.v7.min.js"; \
	else \
	  echo "D3 already present (docs/benchmarks/vendor/d3.v7.min.js)"; \
	fi

bench-serve:
	@echo "Serving repo at http://localhost:8000 (Ctrl+C to stop)"
	@python3 -m http.server 8000

bench-open:
	@open "http://localhost:8000/docs/benchmarks/"

bench-report: vendor-d3
    @echo "Running benches (rmg-benches)..."
    cargo bench -p rmg-benches
    @echo "Starting local server and opening dashboard..."
    @mkdir -p target
    @/bin/sh -c 'python3 -m http.server 8000 >/dev/null 2>&1 & echo $$! > target/bench_http.pid'
    @sleep 1
    @open "http://localhost:8000/docs/benchmarks/"
