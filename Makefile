SHELL := /bin/sh

CHANGE ?= build-tq-mvp
CARGO ?= cargo
OPENSPEC ?= openspec

.DEFAULT_GOAL := help

.PHONY: help fmt fmt-check check lint test openspec-validate \
	preflight-infra preflight engine-gate \
	compatibility-smoke compatibility-full \
	benchmark benchmark-rapid benchmark-smoke benchmark-standard benchmark-large benchmark-extra-large \
	benchmark-refresh-rapid benchmark-refresh-standard benchmark-refresh-large benchmark-refresh-extra-large \
	benchmark-stack-overflow fuzz

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

benchmark: benchmark-rapid ## Run the default rapid benchmark matrix

benchmark-rapid: ## Run the rapid usgs-all-month benchmark matrix
	@./scripts/run-campaign.sh benchmark rapid

benchmark-standard: ## Run standard benchmarks with the machine-local corpus
	@./scripts/run-campaign.sh benchmark standard

benchmark-large: ## Run large benchmarks with the machine-local corpus
	@./scripts/run-campaign.sh benchmark large

benchmark-extra-large: ## Measure selected JSON scaling on the large corpus
	@./scripts/run-campaign.sh benchmark extra-large

benchmark-refresh-standard: ## Refresh upstream standard sources, then benchmark
	@TQ_CORPUS_ORIGIN=refreshed ./scripts/run-campaign.sh benchmark standard

benchmark-refresh-rapid: ## Refresh usgs-all-month, then run the rapid matrix
	@TQ_CORPUS_ORIGIN=refreshed ./scripts/run-campaign.sh benchmark rapid

benchmark-refresh-large: ## Refresh the upstream large source, then benchmark
	@TQ_CORPUS_ORIGIN=refreshed ./scripts/run-campaign.sh benchmark large

benchmark-refresh-extra-large: ## Refresh the large source, then measure parallel scaling
	@TQ_CORPUS_ORIGIN=refreshed ./scripts/run-campaign.sh benchmark extra-large

benchmark-stack-overflow: ## Run the checked-in Stack Overflow jq benchmark
	@./scripts/run-campaign.sh benchmark stack-overflow

fuzz: ## Run bounded parser and differential fuzz checks
	@./scripts/run-campaign.sh fuzz default
