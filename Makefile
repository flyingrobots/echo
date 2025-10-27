SHELL := /bin/bash

.PHONY: hooks
hooks:
	@git config core.hooksPath .githooks
	@chmod +x .githooks/* 2>/dev/null || true
	@echo "[hooks] Installed git hooks from .githooks (core.hooksPath)"

