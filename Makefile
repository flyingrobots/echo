# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

SHELL := /bin/bash

# Default docs port; override with: make docs PORT=5180
PORT ?= 5173
BENCH_PORT ?= 8000

.PHONY: hooks verify-ultra-fast verify-fast verify-pr verify-full verify-full-sequential pr-status pr-snapshot pr-threads pr-preflight docs docs-build docs-ci
hooks:
	@git config core.hooksPath .githooks
	@chmod +x .githooks/* 2>/dev/null || true
	@echo "[hooks] Installed git hooks from .githooks (core.hooksPath)"

verify-ultra-fast:
	@./scripts/verify-local.sh ultra-fast

verify-fast:
	@./scripts/verify-local.sh fast

verify-pr:
	@./scripts/verify-local.sh pr

verify-full:
	@./scripts/verify-local.sh full

verify-full-sequential:
	@VERIFY_LANE_MODE=sequential ./scripts/verify-local.sh full

pr-status:
	@cargo xtask pr-status "$(PR)"

pr-snapshot:
	@cargo xtask pr-snapshot $(ARGS)

pr-threads:
	@cargo xtask pr-threads $(ARGS)

pr-preflight:
	@cargo xtask pr-preflight $(ARGS)

.PHONY: dags dags-fetch
dags:
	@cargo xtask dags

dags-fetch:
	@cargo xtask dags --fetch

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
.PHONY: bench-report bench-vendor vendor-d3 bench-serve bench-open

bench-vendor:
	@mkdir -p docs/benchmarks/vendor
	@if [ ! -f docs/benchmarks/vendor/d3.v7.min.js ]; then \
	  echo "Downloading D3 v7 to docs/benchmarks/vendor..."; \
	  curl -fsSL https://unpkg.com/d3@7/dist/d3.min.js -o docs/benchmarks/vendor/d3.v7.min.js; \
	  echo "D3 saved to docs/benchmarks/vendor/d3.v7.min.js"; \
	else \
	  echo "D3 already present (docs/benchmarks/vendor/d3.v7.min.js)"; \
	fi
	@if [ ! -f docs/benchmarks/vendor/open-props.min.css ]; then \
	  echo "Downloading Open Props to docs/benchmarks/vendor..."; \
	  curl -fsSL https://unpkg.com/open-props@1.7.16/open-props.min.css -o docs/benchmarks/vendor/open-props.min.css; \
	  echo "Open Props saved to docs/benchmarks/vendor/open-props.min.css"; \
	else \
	  echo "Open Props already present (docs/benchmarks/vendor/open-props.min.css)"; \
	fi
	@if [ ! -f docs/benchmarks/vendor/normalize.dark.min.css ]; then \
	  echo "Downloading Open Props normalize.dark to docs/benchmarks/vendor..."; \
	  curl -fsSL https://unpkg.com/open-props@1.7.16/normalize.dark.min.css -o docs/benchmarks/vendor/normalize.dark.min.css; \
	  echo "Open Props normalize.dark saved to docs/benchmarks/vendor/normalize.dark.min.css"; \
	else \
	  echo "Open Props normalize.dark already present (docs/benchmarks/vendor/normalize.dark.min.css)"; \
	fi

vendor-d3: bench-vendor

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

bench-report: bench-vendor
	@echo "Running benches (warp-benches)..."
	cargo bench -p warp-benches
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

.PHONY: bench-bake bench-open-inline bench-policy-bake bench-policy-export bench-policy-open-inline

# Bake an offline-friendly HTML report with inline data and local vendored assets.
bench-bake: bench-vendor
	@echo "Running benches (warp-benches)..."
	cargo bench -p warp-benches
	@echo "Baking inline report..."
	@cargo xtask bench bake \
	  --out docs/benchmarks/report-inline.html \
	  --policy-json-out docs/benchmarks/parallel-policy-matrix.json
	@cargo xtask bench check-artifacts \
	  --html docs/benchmarks/report-inline.html \
	  --json docs/benchmarks/parallel-policy-matrix.json
	@echo "Opening inline report..."
	@open docs/benchmarks/report-inline.html

bench-open-inline:
	@open docs/benchmarks/report-inline.html

bench-policy-export: bench-vendor
	@echo "Exporting parallel policy matrix JSON..."
	@cargo xtask bench policy-export \
	  --json-out docs/benchmarks/parallel-policy-matrix.json
	@echo "Baking unified inline report..."
	@cargo xtask bench bake --out docs/benchmarks/report-inline.html
	@cargo xtask bench check-artifacts \
	  --html docs/benchmarks/report-inline.html \
	  --json docs/benchmarks/parallel-policy-matrix.json
	@pnpm exec prettier --write docs/benchmarks/report-inline.html >/dev/null

bench-policy-bake: bench-vendor
	@echo "Running parallel policy matrix benchmarks..."
	cargo bench -p warp-benches --bench parallel_baseline -- parallel_policy_matrix
	@$(MAKE) bench-policy-export
	@if [ -n "$(OPEN)" ]; then \
	  $(OPEN) "docs/benchmarks/report-inline.html#parallel-policy" >/dev/null 2>&1 || echo "Open file: docs/benchmarks/report-inline.html#parallel-policy" ; \
	else \
	  echo "Open file: docs/benchmarks/report-inline.html#parallel-policy" ; \
	fi

bench-policy-open-inline:
	@if [ -n "$(OPEN)" ]; then \
	  $(OPEN) "docs/benchmarks/report-inline.html#parallel-policy" >/dev/null 2>&1 || echo "Open file: docs/benchmarks/report-inline.html#parallel-policy" ; \
	else \
	  echo "Open file: docs/benchmarks/report-inline.html#parallel-policy" ; \
	fi

# Spec-000 (WASM) helpers
.PHONY: spec-000-dev spec-000-build

spec-000-dev:
	@command -v trunk >/dev/null 2>&1 || { echo "Error: trunk not found. Install: cargo install trunk" >&2; exit 1; }
	@test -d specs/spec-000-rewrite || { echo "Error: specs/spec-000-rewrite not found" >&2; exit 1; }
	@cd specs/spec-000-rewrite && trunk serve

spec-000-build:
	@command -v trunk >/dev/null 2>&1 || { echo "Error: trunk not found. Install: cargo install trunk" >&2; exit 1; }
	@test -d specs/spec-000-rewrite || { echo "Error: specs/spec-000-rewrite not found" >&2; exit 1; }
	@cd specs/spec-000-rewrite && trunk build --release
