# Solana Transfer Substreams

I built a small Rust **Substreams** package that reads Solana blocks and emits **one JSON object per transfer**:

```json
{"from":"SENDER_WALLET","to":"RECIPIENT_WALLET","amount":325253}
```

It covers **native SOL**, **SPL Token**, and **Token-2022**, including both `Transfer` and `TransferChecked`, and resolves SPL token accounts to **wallet owners** when the transaction metadata includes them.

Graded / demo range: Solana mainnet blocks **385870151–385870156** (inclusive).

---

## What I built (plain language)

Think of Solana as a firehose of blocks. Each block contains many transactions; each transaction can contain instructions (including nested “inner” instructions created by programs calling other programs).

This project is a filter + decoder:

1. Connect to a Substreams provider that already has Solana history.
2. Run a tiny WASM module written in Rust over a block range.
3. Keep only successful transfers I care about.
4. Print them as simple JSON lines a reviewer can open in any editor.

```text
 Solana blocks ──► Substreams provider ──► map_transfers (Rust/WASM)
                                              │
                                              ▼
                                    Transfers protobuf
                                              │
                                              ▼
                         flatten script ──► one JSON object per line
```

There is **one map module**, no database, no API server, no custom store pipeline.

---

## Approach & decisions

| Decision | Choice | Why |
|---|---|---|
| Transfer families | Native SOL **and** SPL Token **and** Token-2022 | Assignment asked for transfers; covering all three avoids missing CPI/token activity |
| Instruction variants | `Transfer` + `TransferChecked` | Both are real on-chain transfer forms; Checked is preferred by wallets today |
| SPL `from` / `to` | **Wallet owners** (not raw token accounts) | Reviewer-friendly “who sent / who received” |
| Owner resolution | `pre_token_balances` / `post_token_balances` in tx meta | Works inside a single map module without extra services |
| Failed transactions | Skipped | Failed instructions did not settle |
| Inner instructions | Included via `walk_instructions()` | Many token transfers only appear as CPI |
| Amount | Raw integer (`u64`) | Matches assignment sample (`325253`), lamports or token base units |
| Architecture | Single `map_transfers` module | Assignment scope; simplest correct design |
| Output | Flattened JSONL | Substreams CLI wraps module output; assignment wants one object per transfer |

Out of scope on purpose: `TransferWithSeed`, Token-2022 fee variants, SQL sinks, foundational stores, multi-module graphs.

---

## What I used

| Piece | Role |
|---|---|
| **Rust** → `wasm32-unknown-unknown` | Module language Substreams executes |
| **Substreams CLI** | Auth, protobuf codegen, build `.spkg`, run/gui |
| **Buf** (via CLI) | Generate Rust bindings from my `.proto` |
| **`substreams` + `substreams-solana` crates** | Block/tx/instruction helpers (`walk_instructions`, `is_successful`, resolved accounts) |
| **`substreams-solana-program-instructions`** | Unpack SPL / Token-2022 instruction data |
| **StreamingFast Solana endpoint** | `mainnet.sol.streamingfast.io:443` |
| **Graph Market auth** | `SUBSTREAMS_API_KEY` via `substreams auth` |
| **Python flatten script** | Turn CLI JSON into assignment-shaped JSONL |

Program IDs decoded:

- System: `11111111111111111111111111111111`
- SPL Token: `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`
- Token-2022: `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`

---

## How the full flow works

```text
                  sf.solana.type.v1.Block
                            │
                            ▼
                   for each transaction
                            │
              ┌─────────────┴─────────────┐
              │  skip if meta.err set     │
              └─────────────┬─────────────┘
                            ▼
              build token-account → owner map
              (from pre/post token balances)
                            │
                            ▼
              walk top-level + inner instructions
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
   System Transfer    SPL Transfer/Checked  Token-2022 same
   (lamports)         (resolve owners)      (resolve owners)
        │                   │                   │
        └───────────────────┴───────────────────┘
                            │
                            ▼
                 Transfers { repeated Transfer }
                            │
                            ▼
              substreams run -o jsonl  →  flatten script
                            │
                            ▼
              {"from":"...","to":"...","amount":N}
```

### Native SOL

System Program instruction data starts with discriminator `2` (little-endian u32) followed by an 8-byte lamport amount. Accounts `[0]` / `[1]` are already wallets.

### SPL / Token-2022

Instruction bytes are unpacked with `TokenInstruction`.

- `Transfer` → source / destination token accounts at indices `0` / `1`
- `TransferChecked` → source / destination at `0` / `2` (mint sits in the middle)

I replace token-account addresses with **owners** from balance metadata when available. If a rare account has no owner in meta, I fall back to the token-account address rather than dropping the transfer silently without data.

---

## Project layout

```text
solana-transfer-substreams/
├── README.md
├── Cargo.toml
├── substreams.yaml          # manifest: one map module
├── proto/transfers.proto    # Transfers / Transfer messages
├── src/lib.rs               # map_transfers implementation
├── scripts/
│   ├── run_assignment_range.sh
│   └── flatten_transfers_jsonl.py
└── examples/sample-transfers.jsonl
```

---

## Setup (macOS)

### 1. Toolchain

```bash
# Substreams CLI (binary install also fine)
brew install streamingfast/tap/substreams   # or place binary on PATH

# Rust + wasm target (rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Buf (CLI uses it for codegen)
brew install bufbuild/buf/buf               # or place binary on PATH
```

Confirm:

```bash
substreams --version
rustc --version
rustup target list --installed | grep wasm32-unknown-unknown
buf --version
```

### 2. Authenticate

```bash
cd solana-transfer-substreams
substreams auth          # browser → Graph Market API key
# writes .substreams.env  (gitignored)
source ./.substreams.env
```

Needs a [Graph Market](https://thegraph.market/) account. Prefer `SUBSTREAMS_API_KEY` over short-lived JWT tokens.

### 3. Build

```bash
substreams build
# produces a .spkg and target/wasm32-unknown-unknown/release/substreams.wasm
```

---

## Run the assignment block range

One-shot helper (recommended):

```bash
chmod +x scripts/run_assignment_range.sh
./scripts/run_assignment_range.sh
```

This writes:

- `out/raw-blocks.jsonl` — CLI module output
- `out/transfers.jsonl` — **assignment format**, one object per transfer

Manual equivalent:

```bash
source ./.substreams.env

substreams run -e mainnet.sol.streamingfast.io:443 \
  ./substreams.yaml map_transfers \
  --start-block 385870151 \
  --stop-block 385870157 \
  -o jsonl \
  | tee out/raw-blocks.jsonl \
  | python3 scripts/flatten_transfers_jsonl.py \
  > out/transfers.jsonl
```

Notes:

- `--stop-block` is **exclusive**, so `385870157` includes block `385870156`.
- Interactive debugging: `substreams gui ...` with the same flags.

---

## Output contract

Each line of `out/transfers.jsonl`:

```json
{"from":"<base58 wallet>","to":"<base58 wallet>","amount":<integer>}
```

Example (illustrative):

```json
{"from":"SENDER_BASE58","to":"RECIPIENT_BASE58","amount":325253}
{"from":"AnotherWallet…","to":"RecipientWallet…","amount":100}
```

`amount` is:

- **lamports** for native SOL transfers
- **raw token units** for SPL / Token-2022 (not UI decimals)

---

## Verification checklist for reviewers

1. `substreams build` succeeds.
2. Run covers slots/blocks `385870151`…`385870156` only.
3. `out/transfers.jsonl` is newline-delimited objects with exactly `from`, `to`, `amount`.
4. Spot-check a native SOL transfer and an SPL transfer (if present in range) against a block explorer: wallets and amount match.
5. Failed transactions do not contribute transfers.
6. Re-running the same range yields the same flattened set.

---

## Auth & secrets

- `.substreams.env` is gitignored — never commit API keys.
- Env vars used: `SUBSTREAMS_API_KEY` (preferred), optionally `SUBSTREAMS_API_TOKEN`.

---

## License

Apache-2.0
