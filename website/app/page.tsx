// app/page.tsx
"use client"

import { Page } from '@/components/app-page';
import { fetchMetadata } from '../lib/icp-utils';
import { fetchUserBalances, fetchDctPrice } from '../lib/token-utils';
import { useState, useEffect } from 'react';
import { useAuth } from '@/lib/auth-context';

interface DashboardData {
  dctPrice: number;
  providerCount: number;
  totalBlocks: number;
  blocksUntilHalving: number;
  validatorCount: number;
  blockReward: number;
  userIcpBalance?: number;
  userCkUsdcBalance?: number;
  userCkUsdtBalance?: number;
  userDctBalance?: number;
}

type MetadataValue =
  | { Nat: bigint }
  | { Int: bigint }
  | { Text: string }
  | { Blob: Uint8Array };

type Metadata = Array<[string, MetadataValue]>;

interface UserBalances {
  icp: number;
  ckUsdc: number;
  ckUsdt: number;
  dct: number;
}

function extractDashboardData(metadata: Metadata | null, userBalances?: UserBalances): DashboardData | null {
  if (!metadata) return null;

  const getValue = (key: string): string | number | null => {
    const entry = metadata.find(([k]) => k === key);
    if (!entry) return null;

    const value = entry[1];
    if ('Nat' in value) {
      const num = Number(value.Nat);
      if (key === 'ledger:token_value_in_usd_e6') {
        return num / 1_000_000; // Convert from e6 to actual USD value
      }
      if (key === 'ledger:current_block_rewards_e9s') {
        return num / 1_000_000_000; // Convert from e9s to DCT
      }
      return num;
    }
    if ('Int' in value) return Number(value.Int);
    if ('Text' in value) return value.Text;
    return null;
  };

  const data = {
    dctPrice: getValue('ledger:token_value_in_usd_e6') as number || 0,
    providerCount: getValue('ledger:total_providers') as number || 0,
    totalBlocks: getValue('ledger:num_blocks') as number || 0,
    blocksUntilHalving: getValue('ledger:blocks_until_next_halving') as number || 0,
    validatorCount: getValue('ledger:current_block_validators') as number || 0,
    blockReward: getValue('ledger:current_block_rewards_e9s') as number || 0,
  };

  // Add user balances if available
  if (userBalances) {
    return {
      ...data,
      userIcpBalance: userBalances.icp,
      userCkUsdcBalance: userBalances.ckUsdc,
      userCkUsdtBalance: userBalances.ckUsdt,
      userDctBalance: userBalances.dct,
    };
  }

  return data;
}

export default function HomePage() {
  const [dashboardData, setDashboardData] = useState<DashboardData | null>(null);
  const { isAuthenticated, identity, principal } = useAuth();

  useEffect(() => {
    let mounted = true;

    const fetchData = async () => {
      try {
        const [metadata, dctPrice] = await Promise.all([
          fetchMetadata() as Promise<Metadata>,
          fetchDctPrice()
        ]);

        let userBalances;
        if (isAuthenticated && identity && principal) {
          userBalances = await fetchUserBalances(identity, principal);
        }

        if (mounted) {
          const baseData = extractDashboardData(metadata, userBalances);
          if (baseData) {
            // Override the metadata price with KongSwap price
            baseData.dctPrice = dctPrice;
          }
          setDashboardData(baseData);
        }
      } catch (err) {
        console.error('Error fetching data:', err);
      }
    };

    // Immediate initial fetch
    fetchData().catch(err => {
      if (mounted) {
        console.error('Error in initial data fetch:', err);
      }
    });

    // Set up periodic refresh every 10 seconds
    const intervalId = setInterval(() => {
      fetchData().catch(err => {
        if (mounted) {
          console.error('Error in interval data fetch:', err);
        }
      });
    }, 10000);

    // Cleanup interval and prevent state updates if unmounted
    return () => {
      mounted = false;
      clearInterval(intervalId);
    };
  }, [isAuthenticated, identity, principal]);

  return (
    <Page dashboardData={dashboardData} />
  );
}
