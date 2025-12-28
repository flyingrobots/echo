# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

SHELL := /bin/bash

# Default docs port; override with: make docs PORT=5180
PORT ?= 5173
BENCH_PORT ?= 8000

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
	@echo "Serving repo at http://localhost:$(BENCH_PORT) (Ctrl+C to stop)"
	@python3 -m http.server $(BENCH_PORT)

OPEN := $(shell if command -v open >/dev/null 2>&1; then echo open; \
	elif command -v xdg-open >/dev/null 2>&1; then echo xdg-open; \
	elif command -v powershell.exe >/dev/null 2>&1; then echo powershell.exe; fi)

bench-open:
	@if [ -n "$(OPEN)" ]; then \
	  $(OPEN) "http://localhost:$(BENCH_PORT)/docs/benchmarks/" >/dev/null 2>&1 || echo "Open URL: http://localhost:$(BENCH_PORT)/docs/benchmarks/" ; \
	else \
	  echo "Open URL: http://localhost:$(BENCH_PORT)/docs/benchmarks/" ; \
	fi

bench-report: vendor-d3
	@echo "Running benches (rmg-benches)..."
	cargo bench -p rmg-benches
	@echo "Starting local server on :$(BENCH_PORT) and opening dashboard..."
	@mkdir -p target
	@if [ -f target/bench_http.pid ] && ps -p $$(cat target/bench_http.pid) >/dev/null 2>&1; then \
	  echo "[bench] Stopping previous server (pid $$(cat target/bench_http.pid))"; \
	  kill $$(cat target/bench_http.pid) >/dev/null 2>&1 || true; \
	  rm -f target/bench_http.pid; \
	fi
	@/bin/sh -c 'nohup python3 -m http.server $(BENCH_PORT) >/dev/null 2>&1 & echo $$! > target/bench_http.pid'
	@echo "[bench] Waiting for server to become ready..."
	@for i in {1..80}; do \
	  if curl -sSf "http://localhost:$(BENCH_PORT)/" >/dev/null ; then \
	    echo "[bench] Server is up at http://localhost:$(BENCH_PORT)/" ; \
	    break ; \
	  fi ; \
	  sleep 0.25 ; \
	done
	@if [ -n "$(OPEN)" ]; then \
	  $(OPEN) "http://localhost:$(BENCH_PORT)/docs/benchmarks/" >/dev/null 2>&1 || echo "Open URL: http://localhost:$(BENCH_PORT)/docs/benchmarks/" ; \
	else \
	  echo "Open URL: http://localhost:$(BENCH_PORT)/docs/benchmarks/" ; \
	fi

.PHONY: bench-status bench-stop

bench-status:
	@if [ -f target/bench_http.pid ] && ps -p $$(cat target/bench_http.pid) >/dev/null 2>&1; then \
	  echo "[bench] Server running (pid $$(cat target/bench_http.pid)) at http://localhost:$(BENCH_PORT)"; \
	else \
	  echo "[bench] Server not running"; \
	fi

bench-stop:
	@if [ -f target/bench_http.pid ]; then \
	  kill $$(cat target/bench_http.pid) >/dev/null 2>&1 || true; \
	  rm -f target/bench_http.pid; \
	  echo "[bench] Server stopped"; \
	else \
	  echo "[bench] No PID file at target/bench_http.pid"; \
	fi

.PHONY: bench-bake bench-open-inline

# Bake a standalone HTML with inline data that works over file://
bench-bake: vendor-d3
	@echo "Running benches (rmg-benches)..."
	cargo bench -p rmg-benches
	@echo "Baking inline report..."
	@python3 scripts/bench_bake.py --out docs/benchmarks/report-inline.html
	@echo "Opening inline report..."
	@open docs/benchmarks/report-inline.html

bench-open-inline:
	@open docs/benchmarks/report-inline.html

# Spec-000 (WASM) helpers
.PHONY: spec-000-dev spec-000-build

spec-000-dev:
	@cd specs/spec-000-rewrite && trunk serve

spec-000-build:
	@cd specs/spec-000-rewrite && trunk build --release
