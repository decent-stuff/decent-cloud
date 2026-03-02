#!/usr/bin/env python3
"""PoC: fetch KongSwap token price and convert to USD e6."""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request

KONGSWAP_URL = "https://api.kongswap.io/api/tokens/by_canister"
ICP_LEDGER_CANISTER_ID = "ryjl3-tyaaa-aaaaa-aaaba-cai"


def fetch_token_payload(canister_id: str) -> dict:
    payload = {
        "canister_ids": [canister_id],
        "page": 1,
        "limit": 1,
    }
    body = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        KONGSWAP_URL,
        data=body,
        method="POST",
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            raw = resp.read()
            if resp.status != 200:
                raise RuntimeError(f"HTTP status {resp.status}")
    except urllib.error.URLError as exc:
        raise RuntimeError(f"request failed: {exc}") from exc
    try:
        return json.loads(raw)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"invalid JSON response: {exc}") from exc


def parse_price_usd_to_e6(payload: dict) -> int:
    items = payload.get("items")
    if not isinstance(items, list) or not items:
        raise RuntimeError("No tokens in response")
    price = items[0].get("metrics", {}).get("price")
    if not isinstance(price, (int, float)):
        raise RuntimeError("Missing metrics.price in response")
    if price <= 0:
        raise RuntimeError(f"Invalid price: {price}")
    return round(float(price) * 1_000_000)


def run_happy() -> int:
    payload = fetch_token_payload(ICP_LEDGER_CANISTER_ID)
    price_e6 = parse_price_usd_to_e6(payload)
    print(f"HAPPY: parsed ICP/USD e6={price_e6}")
    return 0


def run_error() -> int:
    payload = fetch_token_payload("aaaaa-aa")
    try:
        parse_price_usd_to_e6(payload)
    except RuntimeError as exc:
        print(f"ERROR_PATH_OK: {exc}")
        return 0
    print("ERROR_PATH_FAILED: expected parser failure for empty items", file=sys.stderr)
    return 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--mode",
        choices=["happy", "error", "both"],
        default="both",
        help="Which PoC flow to run",
    )
    args = parser.parse_args()

    if args.mode == "happy":
        return run_happy()
    if args.mode == "error":
        return run_error()

    rc_happy = run_happy()
    rc_error = run_error()
    return 0 if rc_happy == 0 and rc_error == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
