#!/usr/bin/env bash
# Build (if needed), run the graded block range, and flatten to one JSON object per transfer.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export PATH="${HOME}/.cargo/bin:${HOME}/.local/bin:${PATH}"

ENDPOINT="${ENDPOINT:-mainnet.sol.streamingfast.io:443}"
START_BLOCK="${START_BLOCK:-385870151}"
# stop-block is exclusive → 385870157 includes 385870156
STOP_BLOCK="${STOP_BLOCK:-385870157}"
OUT_DIR="${OUT_DIR:-$ROOT/out}"
RAW_OUT="${RAW_OUT:-$OUT_DIR/raw-blocks.jsonl}"
FLAT_OUT="${FLAT_OUT:-$OUT_DIR/transfers.jsonl}"

mkdir -p "$OUT_DIR"

if [[ -f .substreams.env ]]; then
  # shellcheck disable=SC1091
  set -a
  source ./.substreams.env
  set +a
fi

if [[ -z "${SUBSTREAMS_API_KEY:-}" && -z "${SUBSTREAMS_API_TOKEN:-}" ]]; then
  echo "Missing Substreams credentials." >&2
  echo "Run: substreams auth   # writes .substreams.env with SUBSTREAMS_API_KEY" >&2
  exit 1
fi

echo "==> Building package"
substreams build

echo "==> Running map_transfers on [${START_BLOCK}, ${STOP_BLOCK})"
substreams run -e "$ENDPOINT" ./substreams.yaml map_transfers \
  --start-block "$START_BLOCK" \
  --stop-block "$STOP_BLOCK" \
  -o jsonl \
  > "$RAW_OUT"

echo "==> Flattening to one transfer object per line"
python3 "$ROOT/scripts/flatten_transfers_jsonl.py" < "$RAW_OUT" > "$FLAT_OUT"

echo "==> Done"
echo "Raw CLI output:  $RAW_OUT"
echo "Assignment JSONL: $FLAT_OUT"
wc -l "$FLAT_OUT" | awk '{print "Transfer lines:", $1}'
head -n 5 "$FLAT_OUT" || true
