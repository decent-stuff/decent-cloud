'use client';

import { useState, useEffect } from 'react';
import { useAuth } from '@/lib/auth-context';
import HeaderSection from '@/components/ui/header';
import { motion } from 'framer-motion';
import { useRouter } from 'next/navigation';
import { fetchMetadata } from '@/lib/icp-utils';
import { fetchUserBalances, fetchDctPrice } from '@/lib/token-utils';

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

interface DashboardItem {
  title: string;
  key: keyof DashboardData;
  format: (value: number | undefined) => string;
  tooltip: string;
  showAlsoAnonymous?: boolean;
}

// Using the same dashboard items as in the dashboard-section component
const dashboardItems: DashboardItem[] = [
  {
    title: "Latest DCT Price 💎",
    key: "dctPrice",
    format: (value: number | undefined) => value ? `$${value.toFixed(4)}` : '$0.0000',
    tooltip: "Our token is like a digital diamond: rare, valuable, and utterly decent! Plus, the price updates live from KongSwap.io!",
    showAlsoAnonymous: true,
  },
  {
    title: "Provider Squad 🤝",
    key: "providerCount",
    format: (value: number | undefined) => value ? `${value} providers` : '0 providers',
    tooltip: "Our awesome providers making the cloud decent again!",
    showAlsoAnonymous: true,
  },
  {
    title: "Block Party 🎉",
    key: "totalBlocks",
    format: (value: number | undefined) => value ? value.toLocaleString() : '0',
    tooltip: "Blocks validated and counting!",
    showAlsoAnonymous: true,
  },
  {
    title: "Blocks Until Next Halving ⏳",
    key: "blocksUntilHalving",
    format: (value: number | undefined) => value ? value.toLocaleString() : '0',
    tooltip: "Blocks until rewards halve!",
    showAlsoAnonymous: true,
  },
  {
    title: "Current Block Validators 🛡️",
    key: "validatorCount",
    format: (value: number | undefined) => value ? `${value} validators` : '0 validators',
    tooltip: "Validators keeping us decent!",
    showAlsoAnonymous: true,
  },
  {
    title: "Accumulated Block Rewards 🎁",
    key: "blockReward",
    format: (value: number | undefined) => value ? `${value.toFixed(2)} DCT` : '0.00 DCT',
    tooltip: "DCT per validated block! With carry-over if unclaimed!",
    showAlsoAnonymous: true,
  },
  {
    title: "Your ICP Stash 🏦",
    key: "userIcpBalance",
    format: (value: number | undefined) => value !== undefined ? `${value.toFixed(4)} ICP` : 'Loading...',
    tooltip: "Your Internet Computer tokens!",
    showAlsoAnonymous: false,
  },
  {
    title: "USDC Treasure Chest 💰",
    key: "userCkUsdcBalance",
    format: (value: number | undefined) => value !== undefined ? `${value.toFixed(2)} ckUSDC` : 'Loading...',
    tooltip: "Your ckUSDC balance - stable as a table!",
    showAlsoAnonymous: false,
  },
  {
    title: "USDT Treasure Chest 🏴‍☠️",
    key: "userCkUsdtBalance",
    format: (value: number | undefined) => value !== undefined ? `${value.toFixed(2)} ckUSDT` : 'Loading...',
    tooltip: "Your ckUSDT holdings - just as stable like a table!",
    showAlsoAnonymous: false,
  },
  {
    title: "Your DCT Empire 👑",
    key: "userDctBalance",
    format: (value: number | undefined) => value !== undefined ? `${value.toFixed(2)} DCT` : 'Loading...',
    tooltip: "Your Decent Cloud Tokens - ruling the decentralized clouds!",
    showAlsoAnonymous: false,
  },
];

// Type definitions for metadata
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

// Function to extract dashboard data from metadata
function extractDashboardData(metadata: Metadata | null, userBalances?: UserBalances): DashboardData {
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

export default function DashboardPage() {
  const { isAuthenticated, identity, principal } = useAuth();
  const router = useRouter();
  const [dashboardData, setDashboardData] = useState<DashboardData>({
    dctPrice: 0,
    providerCount: 0,
    totalBlocks: 0,
    blocksUntilHalving: 0,
    validatorCount: 0,
    blockReward: 0,
  });
  const [isLoading, setIsLoading] = useState(true);

  // Check authentication status
  useEffect(() => {
    // Add a small delay to ensure auth state is properly loaded
    const timer = setTimeout(() => {
      setIsLoading(false);
      if (!isAuthenticated) {
        console.log("User not authenticated, redirecting to home page");
        router.push('/');
      } else {
        console.log("User authenticated, staying on dashboard page");
      }
    }, 500);

    return () => clearTimeout(timer);
  }, [isAuthenticated, router]);

  // Fetch dashboard data
  useEffect(() => {
    let mounted = true;

    const fetchData = async () => {
      try {
        console.log("Fetching dashboard data...");
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
        console.error('Error fetching dashboard data:', err);
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

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-white text-xl">Loading dashboard...</div>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <HeaderSection
        title="Dashboard"
        subtitle={
          isAuthenticated
            ? <>
                Welcome back!
                <div className="mt-2 flex items-center justify-center gap-2">
                  <span className="text-sm bg-white/10 px-3 py-1 rounded-lg font-mono">
                    {principal ? principal.toString() : 'Loading...'}
                  </span>
                  <button
                    onClick={() => {
                      if (principal) {
                        navigator.clipboard.writeText(principal.toString())
                          .then(() => {
                            console.log("Principal ID copied to clipboard");
                          })
                          .catch(err => {
                            console.error("Failed to copy principal ID:", err);
                          });
                      }
                    }}
                    className="bg-white/10 hover:bg-white/20 text-white/80 hover:text-white p-1.5 rounded-lg transition-colors"
                    title="Copy Principal ID"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
                    </svg>
                  </button>
                </div>
              </>
            : "Get a quick overview of Decent Cloud's current stats."
        }
      />

      <motion.div
        className="max-w-4xl mx-auto p-6 grid grid-cols-1 sm:grid-cols-2 gap-4"
        initial={{opacity: 0, y: 20}}
        animate={{opacity: 1, y: 0}}
        transition={{duration: 0.5}}
      >
        {dashboardItems
          .filter(item => item.showAlsoAnonymous || isAuthenticated)
          .map((item, index) => (
          <div
            key={index}
            onClick={() => {
              if (item.key === 'userDctBalance') {
                window.open('https://www.kongswap.io/swap?from=ryjl3-tyaaa-aaaaa-aaaba-cai&to=ggi4a-wyaaa-aaaai-actqq-cai', '_blank', 'noopener,noreferrer');
              }
            }}
            className={`border border-white/10 group relative flex flex-col bg-gradient-to-r from-gray-800/30 to-gray-700/30 rounded-xl p-3 sm:p-4 hover:bg-white/10 hover:shadow-xl transition duration-300 ease-in-out shadow-lg ${item.key === 'userDctBalance' ? 'cursor-pointer' : 'cursor-help'}`}
          >
            <div className="font-semibold text-center text-white/90 text-lg sm:text-xl tracking-wide">
              {item.title}
            </div>
            <div className="text-blue-400 font-bold text-xl sm:text-2xl text-center mt-2">
              {item.format(dashboardData[item.key])}
            </div>

            {/* Tooltip */}
            <div
              className="absolute opacity-0 group-hover:opacity-100 transition-opacity duration-300 bg-gray-900 text-white text-xs rounded-lg p-3 shadow-xl border border-white/10
                left-1/2 transform -translate-x-1/2 top-full mt-2 w-56 z-50 pointer-events-none"
            >
              {item.tooltip}
            </div>
          </div>
        ))}
      </motion.div>
    </div>
  );
}
