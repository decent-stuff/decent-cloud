// app/page.tsx
"use client";

import { Page } from "@/components/app-page";
import { fetchMetadata } from "../lib/icp-utils";
import { fetchDctPrice } from "../lib/token-utils";
import { useState, useEffect } from "react";

interface DashboardData {
  dctPrice: number;
  providerCount: number;
  totalBlocks: number;
  blocksUntilHalving: number;
  validatorCount: number;
  blockReward: number;
}

type MetadataValue =
  | { Nat: bigint }
  | { Int: bigint }
  | { Text: string }
  | { Blob: Uint8Array };

type Metadata = Array<[string, MetadataValue]>;

function extractDashboardData(metadata: Metadata | null): DashboardData {
  const defaultData: DashboardData = {
    dctPrice: 0,
    providerCount: 0,
    totalBlocks: 0,
    blocksUntilHalving: 0,
    validatorCount: 0,
    blockReward: 0,
  };

  if (!metadata) return defaultData;

  const getValue = (key: string): string | number | null => {
    const entry = metadata.find(([k]) => k === key);
    if (!entry) return null;

    const value = entry[1];
    if ("Nat" in value) {
      const num = Number(value.Nat);
      if (key === "ledger:token_value_in_usd_e6") {
        return num / 1_000_000; // Convert from e6 to actual USD value
      }
      if (key === "ledger:current_block_rewards_e9s") {
        return num / 1_000_000_000; // Convert from e9s to DCT
      }
      return num;
    }
    if ("Int" in value) return Number(value.Int);
    if ("Text" in value) return value.Text;
    return null;
  };

  return {
    dctPrice: (getValue("ledger:token_value_in_usd_e6") as number) || 0,
    providerCount: (getValue("ledger:total_providers") as number) || 0,
    totalBlocks: (getValue("ledger:num_blocks") as number) || 0,
    blocksUntilHalving:
      (getValue("ledger:blocks_until_next_halving") as number) || 0,
    validatorCount:
      (getValue("ledger:current_block_validators") as number) || 0,
    blockReward: (getValue("ledger:current_block_rewards_e9s") as number) || 0,
  };
}

export default function HomePage() {
  const [dashboardData, setDashboardData] = useState<DashboardData>({
    dctPrice: 0,
    providerCount: 0,
    totalBlocks: 0,
    blocksUntilHalving: 0,
    validatorCount: 0,
    blockReward: 0,
  });

  useEffect(() => {
    let mounted = true;

    const fetchData = async () => {
      try {
        const [metadata, dctPrice] = await Promise.all([
          fetchMetadata() as Promise<Metadata>,
          fetchDctPrice(),
        ]);

        if (mounted) {
          const baseData = extractDashboardData(metadata);
          baseData.dctPrice = dctPrice; // Override with KongSwap price
          setDashboardData(baseData);
        }
      } catch (err) {
        console.error("Error fetching data:", err);
      }
    };

    // Immediate initial fetch
    fetchData().catch((err) => {
      if (mounted) {
        console.error("Error in initial data fetch:", err);
      }
    });

    // Set up periodic refresh every 10 seconds
    const intervalId = setInterval(() => {
      fetchData().catch((err) => {
        if (mounted) {
          console.error("Error in interval data fetch:", err);
        }
      });
    }, 10000);

    // Cleanup interval and prevent state updates if unmounted
    return () => {
      mounted = false;
      clearInterval(intervalId);
    };
  }, []);

  return <Page dashboardData={dashboardData} />;
}
