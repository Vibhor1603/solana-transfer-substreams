.PHONY: build run flatten auth

export PATH := $(HOME)/.cargo/bin:$(HOME)/.local/bin:$(PATH)

ENDPOINT ?= mainnet.sol.streamingfast.io:443
START_BLOCK ?= 385870151
STOP_BLOCK ?= 385870157

auth:
	substreams auth

build:
	substreams build

run: build
	./scripts/run_assignment_range.sh

flatten:
	python3 scripts/flatten_transfers_jsonl.py < out/raw-blocks.jsonl > out/transfers.jsonl
