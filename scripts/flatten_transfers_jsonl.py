#!/usr/bin/env python3
"""Flatten Substreams JSON/JSONL module output into one transfer object per line.

Assignment shape:
  {"from":"...","to":"...","amount":123}

Accepts:
  - raw CLI `-o jsonl` lines (objects that may wrap module data)
  - `-o json` pretty blobs
  - already-flat transfer objects (passed through)
"""

from __future__ import annotations

import json
import sys
from typing import Any, Iterable, Iterator


def iter_json_values(text: str) -> Iterator[Any]:
    text = text.strip()
    if not text:
        return

    # Prefer line-delimited JSON first.
    lines = text.splitlines()
    parsed_any = False
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            yield json.loads(line)
            parsed_any = True
        except json.JSONDecodeError:
            # Fall through to whole-document parse if lines aren't JSONL.
            parsed_any = False
            break

    if parsed_any:
        return

    # Whole-document JSON (possibly concatenated objects).
    decoder = json.JSONDecoder()
    idx = 0
    n = len(text)
    while idx < n:
        while idx < n and text[idx].isspace():
            idx += 1
        if idx >= n:
            break
        obj, end = decoder.raw_decode(text, idx)
        yield obj
        idx = end


def unwrap_data(obj: Any) -> Any:
    if not isinstance(obj, dict):
        return obj
    # Common Substreams CLI wrappers.
    for key in ("data", "@data", "output", "value"):
        if key in obj and isinstance(obj[key], (dict, list)):
            return obj[key]
    return obj


def extract_transfers(obj: Any) -> Iterable[dict[str, Any]]:
    obj = unwrap_data(obj)

    if isinstance(obj, dict):
        # Already a single transfer.
        if set(obj.keys()) >= {"from", "to", "amount"} and "transfers" not in obj:
            yield {
                "from": obj["from"],
                "to": obj["to"],
                "amount": coerce_amount(obj["amount"]),
            }
            return

        transfers = obj.get("transfers")
        if isinstance(transfers, list):
            for item in transfers:
                if isinstance(item, dict) and "from" in item and "to" in item and "amount" in item:
                    yield {
                        "from": item["from"],
                        "to": item["to"],
                        "amount": coerce_amount(item["amount"]),
                    }
            return

    if isinstance(obj, list):
        for item in obj:
            yield from extract_transfers(item)


def coerce_amount(value: Any) -> int:
    if isinstance(value, bool):
        raise ValueError(f"invalid amount: {value!r}")
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    if isinstance(value, str):
        return int(value)
    raise ValueError(f"invalid amount: {value!r}")


def main() -> int:
    text = sys.stdin.read()
    count = 0
    for value in iter_json_values(text):
        for transfer in extract_transfers(value):
            sys.stdout.write(json.dumps(transfer, separators=(",", ":"), ensure_ascii=False))
            sys.stdout.write("\n")
            count += 1
    print(f"# flattened {count} transfer(s)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
