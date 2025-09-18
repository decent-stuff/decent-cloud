"use client";
import { useState, useEffect } from "react";
import { useAuth, IdentityInfo } from "@/lib/auth-context";
import HeaderSection from "@/components/ui/header";
import { Button } from "@/components/ui/button";
import Link from "next/link";
import { motion } from "framer-motion";
import { fetchMetadata } from "@/lib/icp-utils";
import { fetchUserBalances, fetchDctPrice } from "@/lib/token-utils";
import { sendFunds, getTopUpUrl } from "@/lib/send-funds-utils";
import { SendFundsDialog } from "@/components/send-funds-dialog";
import { SeedPhraseDialog } from "@/components/seed-phrase-dialog";
import { Principal } from "@dfinity/principal";

// Identity badge configuration
const badgeStyles = {
  current: {
    bg: "bg-blue-900/30",
    border: "border-blue-500/30",
    text: "text-blue-400",
    label: "Current"
  },
  signing: {
    bg: "bg-emerald-900/30",
    border: "border-emerald-500/30",
    text: "text-emerald-400",
    label: "Signing"
  },
};

interface IdentityBadgesProps {
  isCurrent: boolean;
  isSigning: boolean;
  type: IdentityInfo["type"];
}

const IdentityBadges = ({ isCurrent, isSigning }: IdentityBadgesProps) => {
  const badges = [
    { show: isCurrent, style: badgeStyles.current },
    { show: isSigning, style: badgeStyles.signing },
  ];

  return (
    <div className="flex gap-2">
      {badges.map(({ show, style }, index) =>
        show ? (
          <span
            key={index}
            className={`text-xs ${style.bg} border ${style.border} px-2 py-1 rounded ${style.text}`}
          >
            {style.label}
          </span>
        ) : null
      )}
    </div>
  );
};

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
    format: (value: number | undefined) =>
      value ? `$${value.toFixed(4)}` : "$0.0000",
    tooltip:
      "Our token is like a digital diamond: rare, valuable, and utterly decent! Plus, the price updates live from KongSwap.io!",
    showAlsoAnonymous: true,
  },
  {
    title: "Provider Squad 🤝",
    key: "providerCount",
    format: (value: number | undefined) =>
      value ? `${value} providers` : "0 providers",
    tooltip: "Our awesome providers making the cloud decent again!",
    showAlsoAnonymous: true,
  },
  {
    title: "Block Party 🎉",
    key: "totalBlocks",
    format: (value: number | undefined) =>
      value ? value.toLocaleString() : "0",
    tooltip: "Blocks validated and counting!",
    showAlsoAnonymous: true,
  },
  {
    title: "Blocks Until Next Halving ⏳",
    key: "blocksUntilHalving",
    format: (value: number | undefined) =>
      value ? value.toLocaleString() : "0",
    tooltip: "Blocks until rewards halve!",
    showAlsoAnonymous: true,
  },
  {
    title: "Current Block Validators 🛡️",
    key: "validatorCount",
    format: (value: number | undefined) =>
      value ? `${value} validators` : "0 validators",
    tooltip: "Validators keeping us decent!",
    showAlsoAnonymous: true,
  },
  {
    title: "Accumulated Block Rewards 🎁",
    key: "blockReward",
    format: (value: number | undefined) =>
      value ? `${value.toFixed(2)} DCT` : "0.00 DCT",
    tooltip: "DCT per validated block! With carry-over if unclaimed!",
    showAlsoAnonymous: true,
  },
  {
    title: "Your ICP Stash 🏦",
    key: "userIcpBalance",
    format: (value: number | undefined) =>
      value !== undefined ? `${value.toFixed(4)} ICP` : "Loading...",
    tooltip: "Your Internet Computer tokens!",
    showAlsoAnonymous: false,
  },
  {
    title: "USDC Treasure Chest 💰",
    key: "userCkUsdcBalance",
    format: (value: number | undefined) =>
      value !== undefined ? `${value.toFixed(2)} ckUSDC` : "Loading...",
    tooltip: "Your ckUSDC balance - stable as a table!",
    showAlsoAnonymous: false,
  },
  {
    title: "USDT Treasure Chest 🏴‍☠️",
    key: "userCkUsdtBalance",
    format: (value: number | undefined) =>
      value !== undefined ? `${value.toFixed(2)} ckUSDT` : "Loading...",
    tooltip: "Your ckUSDT holdings - just as stable like a table!",
    showAlsoAnonymous: false,
  },
  {
    title: "Your DCT Empire 👑",
    key: "userDctBalance",
    format: (value: number | undefined) =>
      value !== undefined ? `${value.toFixed(2)} DCT` : "Loading...",
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
function extractDashboardData(
  metadata: Metadata | null,
  userBalances?: UserBalances
): DashboardData {
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

  const data = {
    dctPrice: (getValue("ledger:token_value_in_usd_e6") as number) || 0,
    providerCount: (getValue("ledger:total_providers") as number) || 0,
    totalBlocks: (getValue("ledger:num_blocks") as number) || 0,
    blocksUntilHalving:
      (getValue("ledger:blocks_until_next_halving") as number) || 0,
    validatorCount:
      (getValue("ledger:current_block_validators") as number) || 0,
    blockReward: (getValue("ledger:current_block_rewards_e9s") as number) || 0,
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
  const {
    isAuthenticated,
    currentIdentity,
    signingIdentity,
    identities,
    switchIdentity,
    signOutIdentity,
    errorMessage,
    backupSeedPhrase
  } = useAuth();

  const [dashboardData, setDashboardData] = useState<DashboardData>({
    dctPrice: 0,
    providerCount: 0,
    totalBlocks: 0,
    blocksUntilHalving: 0,
    validatorCount: 0,
    blockReward: 0,
  });

  // State for send funds dialog
  const [sendFundsDialogOpen, setSendFundsDialogOpen] = useState(false);
  const [showBackupDialog, setShowBackupDialog] = useState(false);
  const [backupIdentity, setBackupIdentity] = useState<Principal | null>(null);
  const [selectedToken, setSelectedToken] = useState<{
    name: string;
    balance?: number;
    key: keyof DashboardData;
  } | null>(null);

  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [identityToSignOut, setIdentityToSignOut] = useState<Principal | null>(
    null
  );

  // Dialog state for sign out confirmation
  const [showSignOutDialog, setShowSignOutDialog] = useState(false);

  // Fetch dashboard data
  useEffect(() => {
    let mounted = true;

    const fetchData = async () => {
      try {
        console.log("Fetching dashboard data...");
        const [metadata, dctPrice] = await Promise.all([
          fetchMetadata() as Promise<Metadata>,
          fetchDctPrice(),
        ]);

        let userBalances;
        if (isAuthenticated && currentIdentity) {
          userBalances = await fetchUserBalances(
            currentIdentity.identity,
            currentIdentity.principal
          );
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
        console.error("Error fetching dashboard data:", err);
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
  }, [isAuthenticated, currentIdentity]);

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="mb-8">
        <HeaderSection
          title="Dashboard"
          subtitle={
            isAuthenticated
              ? "Your personal Decent Cloud dashboard!"
              : "Please sign in to see your personal dashboard."
          }
        />
      </div>

      <div className="flex flex-col sm:flex-row justify-end gap-3 mb-8">
        <Link href={`/login?returnUrl=${encodeURIComponent("/dashboard")}`}>
          <Button
            variant="outline"
            className="bg-white/10 text-white hover:bg-white/20 flex items-center gap-2"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="h-4 w-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M11 16l-4-4m0 0l4-4m-4 4h14m-5 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h7a3 3 0 013 3v1"
              />
            </svg>
            {isAuthenticated ? "Add Identity" : "Register/Sign In"}
          </Button>
        </Link>
        {isAuthenticated && currentIdentity && (
          <>
            <Button
              onClick={() => signOutIdentity(currentIdentity.principal)}
              variant="outline"
              className="bg-white/10 text-white hover:bg-white/20 flex items-center gap-2"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"
                />
              </svg>
              Sign Out
            </Button>
          </>
        )}
      </div>

      {/* Identity List (only when authenticated) */}
      {isAuthenticated && identities.length > 0 && (
        <motion.div
          className="mb-8 bg-gradient-to-r from-gray-800/50 to-gray-700/50 rounded-xl p-4 border border-white/10 shadow-lg"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.3 }}
        >
          <div className="flex flex-col gap-4">
            <div className="flex justify-between items-center mb-4">
              <h3 className="text-white/80 text-sm font-medium">Your Identities</h3>
            </div>
            {errorMessage && (
              <div className="mb-4 p-3 bg-red-900/30 border border-red-500/30 rounded-lg text-red-400 text-sm">
                {errorMessage}
              </div>
            )}
            <div className="grid gap-3">
              {identities.map((identity) => {
                const isCurrent =
                  currentIdentity?.principal.toString() ===
                  identity.principal.toString();
                return (
                  <div
                    key={identity.principal.toString()}
                    className={`w-full text-left p-3 rounded-lg transition-colors flex flex-col sm:flex-row sm:items-center justify-between gap-3 ${
                      isCurrent
                        ? "bg-white/20 border-2 border-white/30"
                        : "bg-white/10 hover:bg-white/15 border border-white/10"
                    }`}
                  >
                    <button
                      onClick={() => switchIdentity(identity.principal)}
                      className="flex items-center gap-3 flex-1 text-left"
                    >
                      <div
                        className={`text-lg ${
                          isCurrent ? "text-white" : "text-white/70"
                        }`}
                      >
                        {identity.type === "ii" ? "🔑" : "🌱"}
                      </div>
                      <div>
                        <div
                          className={`font-medium ${
                            isCurrent ? "text-white" : "text-white/90"
                          }`}
                        >
                          {identity.type === "ii"
                            ? "Internet Identity"
                            : "Seed Phrase"}
                        </div>
                        <div className="text-sm font-mono text-white/60">
                          {identity.principal.toString()}
                        </div>
                      </div>
                    </button>
                    <div className="flex items-center gap-2">
                      <div className="flex gap-2">
                        {identity.type === "seedPhrase" && (
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              setBackupIdentity(identity.principal);
                              setShowBackupDialog(true);
                            }}
                            className="text-amber-400 hover:text-amber-300 transition-colors"
                            title="Show Backup Instructions"
                          >
                            💾
                          </button>
                        )}
                        <IdentityBadges
                          isCurrent={identity.principal.toString() === currentIdentity?.principal.toString()}
                          isSigning={identity.principal.toString() === signingIdentity?.principal.toString()}
                          type={identity.type}
                        />
                      </div>
                      <button
                        onClick={() => {
                          navigator.clipboard
                            .writeText(identity.principal.toString())
                            .then(() => {
                              console.log("Principal ID copied to clipboard");
                              setCopiedId(identity.principal.toString());
                              setTimeout(() => setCopiedId(null), 2000);
                            })
                            .catch((err) => {
                              console.error(
                                "Failed to copy principal ID:",
                                err
                              );
                            });
                        }}
                        className="text-white/60 hover:text-white/90 transition-colors"
                        title="Copy Principal ID"
                      >
                        {copiedId === identity.principal.toString()
                          ? "✓"
                          : "📋"}
                      </button>
                      <button
                        onClick={() => {
                          setIdentityToSignOut(identity.principal);
                          setShowSignOutDialog(true);
                        }}
                        className="text-white/60 hover:text-red-400 transition-colors"
                        title="Sign Out"
                      >
                        ⏻
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </motion.div>
      )}

      {/* Sign Out Confirmation Dialog */}
      {showSignOutDialog && identityToSignOut && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center p-4 z-50">
          <div className="bg-gray-800 rounded-xl p-6 max-w-md w-full border border-white/10">
            <h3 className="text-xl font-medium text-white mb-4">
              Confirm Sign Out
            </h3>
            <p className="text-white/80 mb-6">
              Are you sure you want to sign out this identity? You'll need to
              sign in again to use it.
            </p>
            <div className="flex justify-end gap-3">
              <button
                onClick={() => {
                  setShowSignOutDialog(false);
                  setIdentityToSignOut(null);
                }}
                className="px-4 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white/90 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={() => {
                  if (identityToSignOut) {
                    signOutIdentity(identityToSignOut);
                    setShowSignOutDialog(false);
                    setIdentityToSignOut(null);
                  }
                }}
                className="px-4 py-2 rounded-lg bg-red-500/80 hover:bg-red-500 text-white transition-colors"
              >
                Sign Out
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Network Stats Section */}
      <motion.div
        className="mb-8"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
      >
        <h2 className="text-xl font-bold text-blue-300 mb-4 border-b border-white/10 pb-2">
          Network Statistics
        </h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {dashboardItems
            .filter((item) => item.showAlsoAnonymous)
            .map((item, index) => (
              <div
                key={index}
                className="group relative flex flex-col bg-gradient-to-r from-gray-800/40 to-gray-700/40 rounded-xl p-4 hover:bg-white/10 hover:shadow-xl transition duration-300 ease-in-out shadow-lg border border-white/10 cursor-help"
              >
                <div className="font-medium text-white/80 text-base mb-1">
                  {item.title}
                </div>
                <div className="text-blue-400 font-bold text-xl mt-1">
                  {item.format(dashboardData[item.key])}
                </div>

                <div
                  className="absolute opacity-0 group-hover:opacity-100 transition-opacity duration-300 bg-gray-900/95 text-white text-xs rounded-lg p-3 shadow-xl border border-white/20
                  left-1/2 transform -translate-x-1/2 top-full mt-2 w-56 z-50 pointer-events-none backdrop-blur-sm"
                >
                  {item.tooltip}
                </div>
              </div>
            ))}
        </div>
      </motion.div>

      {/* User Balances Section - Only shown when authenticated */}
      {isAuthenticated && (
        <motion.div
          className="mb-8"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.1 }}
        >
          <h2 className="text-xl font-bold text-emerald-300 mb-4 border-b border-white/10 pb-2">
            Your Balances
          </h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {dashboardItems
              .filter((item) => !item.showAlsoAnonymous)
              .map((item, index) => {
                // Determine token type based on the item key
                const tokenType =
                  item.key === "userDctBalance"
                    ? "DCT"
                    : item.key === "userIcpBalance"
                    ? "ICP"
                    : item.key === "userCkUsdcBalance"
                    ? "USDC"
                    : "USDT";

                // Get token display name
                const tokenName =
                  item.key === "userDctBalance"
                    ? "DCT"
                    : item.key === "userIcpBalance"
                    ? "ICP"
                    : item.key === "userCkUsdcBalance"
                    ? "ckUSDC"
                    : "ckUSDT";

                return (
                  <div
                    key={index}
                    className={`group relative flex flex-col bg-gradient-to-r
                      ${
                        item.key === "userDctBalance"
                          ? "from-purple-900/30 to-blue-900/30 border-purple-500/30"
                          : item.key === "userIcpBalance"
                          ? "from-amber-900/30 to-orange-900/30 border-amber-500/30"
                          : "from-teal-900/30 to-emerald-900/30 border-teal-500/30"
                      }
                      rounded-xl p-4 hover:bg-white/10 hover:shadow-xl transition duration-300 ease-in-out shadow-lg border border-white/10`}
                  >
                    <div className="flex justify-between items-start">
                      <div className="font-medium text-white/80 text-base mb-1">
                        {item.title}
                      </div>
                    </div>
                    <div
                      className={`font-bold text-xl mt-1 ${
                        item.key === "userDctBalance"
                          ? "text-purple-400"
                          : item.key === "userIcpBalance"
                          ? "text-amber-400"
                          : "text-teal-400"
                      }`}
                    >
                      {item.format(dashboardData[item.key])}
                    </div>

                    {/* Action buttons */}
                    <div className="flex gap-2 mt-3">
                      <Button
                        size="sm"
                        variant="outline"
                        className="bg-white/5 hover:bg-white/10 text-white/80 hover:text-white border-white/10 text-xs flex-1"
                        onClick={() => {
                          setSelectedToken({
                            name: tokenName,
                            balance: dashboardData[item.key] as number,
                            key: item.key,
                          });
                          setSendFundsDialogOpen(true);
                        }}
                      >
                        <svg
                          xmlns="http://www.w3.org/2000/svg"
                          className="h-3 w-3 mr-1"
                          fill="none"
                          viewBox="0 0 24 24"
                          stroke="currentColor"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
                          />
                        </svg>
                        Send
                      </Button>

                      <Link
                        href={getTopUpUrl(
                          tokenType as "ICP" | "USDT" | "USDC" | "DCT"
                        )}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="flex-1"
                      >
                        <Button
                          size="sm"
                          variant="outline"
                          className={`w-full bg-white/5 hover:bg-white/10 text-white/80 hover:text-white border-white/10 text-xs`}
                        >
                          <svg
                            xmlns="http://www.w3.org/2000/svg"
                            className="h-3 w-3 mr-1"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                          >
                            <path
                              strokeLinecap="round"
                              strokeLinejoin="round"
                              strokeWidth={2}
                              d="M12 6v6m0 0v6m0-6h6m-6 0H6"
                            />
                          </svg>
                          Top Up
                        </Button>
                      </Link>
                    </div>

                    <div
                      className="absolute opacity-0 group-hover:opacity-100 transition-opacity duration-300 bg-gray-900/95 text-white text-xs rounded-lg p-3 shadow-xl border border-white/20
                      left-1/2 transform -translate-x-1/2 top-full mt-2 w-56 z-50 pointer-events-none backdrop-blur-sm"
                    >
                      {item.tooltip}
                    </div>
                  </div>
                );
              })}
          </div>
        </motion.div>
      )}

      {/* Send Funds Dialog */}
      {selectedToken && (
        <SendFundsDialog
          isOpen={sendFundsDialogOpen}
          onClose={() => {
            setSendFundsDialogOpen(false);
          }}
          onSend={async (recipient, amount) => {
            if (!currentIdentity) return;

            // Determine token type
            const tokenType =
              selectedToken.key === "userDctBalance"
                ? "DCT"
                : selectedToken.key === "userIcpBalance"
                ? "ICP"
                : selectedToken.key === "userCkUsdcBalance"
                ? "USDC"
                : "USDT";

            // Call the stub function
            const result = await sendFunds({
              recipient: recipient,
              amount,
              tokenType: tokenType as "ICP" | "USDT" | "USDC" | "DCT",
              identity: currentIdentity.identity,
              principal: currentIdentity.principal,
            });

            // Show status message
            if (result.success) {
              alert(`✅ Success: ${result.message}`);
            } else {
              alert(`❌ Error: ${result.message}`);
            }
          }}
          tokenName={selectedToken.name}
          maxAmount={selectedToken.balance}
        />
      )}
      {/* Backup Seed Phrase Dialog */}
      <SeedPhraseDialog
        isOpen={showBackupDialog}
        onClose={() => {
          setShowBackupDialog(false);
          setBackupIdentity(null);
        }}
        mode="backup"
        withSeedPhrase={backupIdentity ? backupSeedPhrase(backupIdentity) : null}
      />
    </div>
  );
}
