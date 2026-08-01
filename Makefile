SHELL := /bin/sh

CHANGE ?= build-tq-mvp
CARGO ?= cargo
OPENSPEC ?= openspec

.DEFAULT_GOAL := help

.PHONY: help fmt fmt-check check lint test openspec-validate \
	preflight-infra preflight engine-gate \
	compatibility-smoke compatibility-full \
	benchmark-smoke benchmark-standard benchmark-large fuzz

help:
	@awk 'BEGIN {FS = ":.*## "}; /^[a-zA-Z0-9_-]+:.*## / {printf "%-24s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

fmt: ## Format all Rust workspace members
	$(CARGO) fmt --all

fmt-check: ## Check Rust formatting without modifying files
	$(CARGO) fmt --all --check

check: ## Type-check all workspace targets
	$(CARGO) check --workspace --all-targets

lint: ## Run workspace Clippy policy as errors
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

test: ## Run all workspace tests
	$(CARGO) test --workspace

openspec-validate: ## Strictly validate the active OpenSpec change
	OPENSPEC_TELEMETRY=0 $(OPENSPEC) validate $(CHANGE) --strict

preflight-infra: fmt-check check lint test openspec-validate ## Validate infrastructure without requiring a tq engine

preflight: preflight-infra ## Local and future automation entry point

engine-gate: ## Require completion of baseline tasks in sections 2-5
	@./scripts/check-baseline-gate.sh

compatibility-smoke: ## Run the checked-in compatibility smoke suite
	@./scripts/run-campaign.sh compatibility smoke

compatibility-full: ## Run the complete compatibility suite
	@./scripts/run-campaign.sh compatibility full

benchmark-smoke: ## Run checked-in smoke-corpus benchmarks
	@./scripts/run-campaign.sh benchmark smoke

benchmark-standard: ## Run refreshed standard-size benchmarks
	@./scripts/run-campaign.sh benchmark standard

benchmark-large: ## Run refreshed natural large-file benchmarks
	@./scripts/run-campaign.sh benchmark large

fuzz: ## Run bounded parser and differential fuzz checks
	@./scripts/run-campaign.sh fuzz default
