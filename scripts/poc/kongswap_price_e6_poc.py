#!/usr/bin/env python3
"""PoC: query KongSwap backend canister and convert pool price to USD e6."""

from __future__ import annotations

import argparse
import math
import os
import re
import subprocess
import sys

KONGSWAP_CANISTER_ID = "2ipq2-uqaaa-aaaar-qailq-cai"
ICP_LEDGER_CANISTER_ID = "ryjl3-tyaaa-aaaaa-aaaba-cai"
CKUSDT_LEDGER_CANISTER_ID = "cngnf-vqaaa-aaaar-qag4q-cai"


def fetch_pool_output(token_canister_id: str) -> str:
    pool_filter = f"{token_canister_id}_{CKUSDT_LEDGER_CANISTER_ID}"
    candid_arg = f'(opt "{pool_filter}")'
    try:
        result = subprocess.run(
            [
                "dfx",
                "canister",
                "--network",
                "ic",
                "call",
                KONGSWAP_CANISTER_ID,
                "pools",
                candid_arg,
            ],
            check=True,
            capture_output=True,
            text=True,
            env={**dict(os.environ), "DFX_WARNING": "-mainnet_plaintext_identity"},
            timeout=30,
        )
    except subprocess.CalledProcessError as exc:
        stderr = exc.stderr.strip()
        raise RuntimeError(f"dfx canister call failed: {stderr}") from exc
    except subprocess.TimeoutExpired as exc:
        raise RuntimeError("dfx canister call timed out") from exc

    return result.stdout


def parse_price_usd_to_e6(output: str, token_canister_id: str) -> int:
    if "variant { Err" in output:
        raise RuntimeError("KongSwap returned Err variant")

    pattern = re.compile(
        r'address_0\s*=\s*"([^"]+)";\s*'
        r'address_1\s*=\s*"([^"]+)";[\s\S]*?'
        r"price\s*=\s*([-+]?[0-9]*\.?[0-9]+)\s*:\s*float64;[\s\S]*?"
        r"is_removed\s*=\s*(true|false);",
        re.MULTILINE,
    )

    for match in pattern.finditer(output):
        address_0, address_1, price_text, is_removed_text = match.groups()
        is_removed = is_removed_text == "true"
        if is_removed:
            continue
        if address_0 != token_canister_id or address_1 != CKUSDT_LEDGER_CANISTER_ID:
            continue
        price = float(price_text)
        if not math.isfinite(price) or price <= 0:
            raise RuntimeError(f"Invalid pool price: {price}")
        return round(price * 1_000_000)

    raise RuntimeError(f"No active KongSwap pool found for {token_canister_id}_{CKUSDT_LEDGER_CANISTER_ID}")


def run_happy() -> int:
    output = fetch_pool_output(ICP_LEDGER_CANISTER_ID)
    price_e6 = parse_price_usd_to_e6(output, ICP_LEDGER_CANISTER_ID)
    print(f"HAPPY: parsed ICP/ckUSDT e6={price_e6}")
    return 0


def run_error() -> int:
    output = fetch_pool_output("aaaaa-aa")
    try:
        parse_price_usd_to_e6(output, "aaaaa-aa")
    except RuntimeError as exc:
        print(f"ERROR_PATH_OK: {exc}")
        return 0
    print("ERROR_PATH_FAILED: expected parser failure for missing pool", file=sys.stderr)
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
