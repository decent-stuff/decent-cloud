"use client";

import { useState, useEffect } from "react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faTrophy,
  faMedal,
  faAward,
  faSort,
  faSortUp,
  faSortDown,
  faSync,
} from "@fortawesome/free-solid-svg-icons";
import { ledgerService, ValidatorInfo } from "@/lib/ledger-service";
import HeaderSection from "@/components/ui/header";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ValidationResult } from "@/lib/blockchain-validator";
import { BlockchainValidator } from "@/components/blockchain-validator";

type SortField = "blocksValidated" | "rewards" | "stake";
type SortDirection = "asc" | "desc";

export default function ValidatorsPage() {
  const [validators, setValidators] = useState<ValidatorInfo[]>([]);
  const [sortField, setSortField] = useState<SortField>("blocksValidated");
  const [sortDirection, setSortDirection] = useState<SortDirection>("desc");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  // We track validation result but don't display it directly as the BlockchainValidator component handles that
  const [, setValidationResult] = useState<ValidationResult | null>(null);

  // Initialize ledger service and fetch validators
  useEffect(() => {
    async function initAndFetchValidators() {
      try {
        setIsLoading(true);
        setError(null);

        // Initialize ledger service
        const initialized = await ledgerService.initialize();
        if (!initialized) {
          setError("Failed to initialize ledger service");
          return;
        }

        // Start polling for updates
        await ledgerService.startPolling();

        // Fetch validators
        const validatorData = await ledgerService.getValidators();
        setValidators(validatorData);
      } catch (err) {
        console.error("Error fetching validators:", err);
        setError(
          err instanceof Error ? err.message : "Failed to fetch validators"
        );
      } finally {
        setIsLoading(false);
      }
    }

    void initAndFetchValidators();

    // Clean up on unmount
    return () => {
      ledgerService.stopPolling();
    };
  }, []);

  // Get sorted validators based on current sort parameters
  const sortedValidators = [...validators].sort((a, b) => {
    if (sortDirection === "asc") {
      return a[sortField] - b[sortField];
    } else {
      return b[sortField] - a[sortField];
    }
  });

  // Refresh validators data
  const refreshValidators = async () => {
    try {
      setIsLoading(true);
      setError(null);

      // Fetch latest ledger blocks
      await ledgerService.fetchAndStoreLatestEntries();

      // Get updated validators
      const validatorData = await ledgerService.getValidators();
      setValidators(validatorData);
    } catch (err) {
      console.error("Error refreshing validators:", err);
      setError(
        err instanceof Error ? err.message : "Failed to refresh validators"
      );
    } finally {
      setIsLoading(false);
    }
  };

  const handleSort = (field: SortField) => {
    if (field === sortField) {
      // Toggle direction if same field
      setSortDirection(sortDirection === "asc" ? "desc" : "asc");
    } else {
      // New field, default to descending
      setSortField(field);
      setSortDirection("desc");
    }
  };

  const getSortIcon = (field: SortField) => {
    if (field !== sortField)
      return <FontAwesomeIcon icon={faSort} className="ml-1 text-white/50" />;
    return sortDirection === "asc" ? (
      <FontAwesomeIcon icon={faSortUp} className="ml-1 text-blue-400" />
    ) : (
      <FontAwesomeIcon icon={faSortDown} className="ml-1 text-blue-400" />
    );
  };

  const handleValidationComplete = (result: ValidationResult) => {
    setValidationResult(result);
    // Refresh validators after successful validation
    if (result.success) {
      void refreshValidators();
    }
  };

  // Format principal ID for display
  const formatPrincipal = (principal: string) => {
    if (principal.length <= 15) return principal;
    return `${principal.substring(0, 10)}...${principal.substring(
      principal.length - 5
    )}`;
  };

  // Loading state
  if (isLoading && validators.length === 0) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-white text-xl">Loading validators data...</div>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <HeaderSection title="Validation Dashboard" />

      {error && (
        <div className="bg-red-900/30 p-4 rounded-lg mb-6 text-white">
          <p className="font-semibold">Error loading validator data:</p>
          <p>{error}</p>
          <Button
            onClick={() => {
              refreshValidators().catch(console.error);
            }}
            className="mt-2 bg-blue-600 hover:bg-blue-700 text-white"
          >
            <FontAwesomeIcon icon={faSync} className="mr-2" />
            Retry
          </Button>
        </div>
      )}

      <div className="bg-white/10 p-6 rounded-lg backdrop-blur-sm mb-6">
        {/* Blockchain Validation Section */}
        <div className="bg-white/10 p-6 rounded-lg backdrop-blur-sm">
          <h3 className="text-xl font-semibold mb-2 text-white">
            Blockchain Validation
          </h3>
          <p className="text-white/90 mb-4">
            As a validator, you can participate in blockchain validation by
            checking in with the network. This helps maintain the integrity and
            security of the Decent Cloud network.
          </p>

          <BlockchainValidator
            defaultMemo="Website Validation works!!1!"
            darkMode={true}
            renderAsCard={false}
            onValidationComplete={handleValidationComplete}
          />
        </div>

        {sortedValidators.length > 0 && (
          <div className="mt-6">
            <div className="flex justify-between items-center mb-4">
              <h3 className="text-xl font-semibold text-white">
                Top Validators
              </h3>
              <Button
                onClick={() => {
                  refreshValidators().catch(console.error);
                }}
                className="bg-blue-600 hover:bg-blue-700 text-white"
                disabled={isLoading}
              >
                <FontAwesomeIcon icon={faSync} className="mr-2" />
                {isLoading ? "Refreshing..." : "Refresh Data"}
              </Button>
            </div>

            {/* Top 3 Validators */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
              {sortedValidators.slice(0, 3).map((validator, index) => (
                <div
                  key={validator.principal}
                  className="border border-white/10 rounded-lg p-4 bg-gradient-to-b from-white/10 to-white/5 flex flex-col items-center"
                >
                  <div className="mb-2">
                    {index === 0 ? (
                      <FontAwesomeIcon
                        icon={faTrophy}
                        className="text-yellow-400 text-3xl"
                      />
                    ) : index === 1 ? (
                      <FontAwesomeIcon
                        icon={faMedal}
                        className="text-gray-400 text-3xl"
                      />
                    ) : (
                      <FontAwesomeIcon
                        icon={faAward}
                        className="text-amber-700 text-3xl"
                      />
                    )}
                  </div>
                  <h4 className="text-lg font-medium text-white mb-1">
                    {validator.name || `Validator ${index + 1}`}
                  </h4>
                  <p className="text-white/70 text-sm mb-2 truncate max-w-full">
                    {formatPrincipal(validator.principal)}
                  </p>
                  <div className="text-blue-400 font-bold text-xl mb-2">
                    {validator.blocksValidated.toLocaleString()} blocks
                  </div>
                  <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm w-full">
                    <span className="text-white/70">Rewards:</span>
                    <span className="text-white text-right">
                      {validator.rewards.toLocaleString()} DCT
                    </span>
                    <span className="text-white/70">Last Memo:</span>
                    <span
                      className="text-white text-right truncate max-w-[120px]"
                      title={validator.memo}
                    >
                      {validator.memo || "N/A"}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <Card className="p-6 bg-white/10 backdrop-blur-sm rounded-lg border border-white/20">
        <div className="flex justify-between items-center mb-4">
          <h3 className="text-xl font-semibold text-white">All Validators</h3>
          <div className="text-xs text-white/70 bg-blue-500/20 px-3 py-1 rounded-full">
            {sortedValidators.length} validators
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-white">
            <thead>
              <tr className="border-b border-white/20">
                <th className="py-3 px-4 text-left">Rank</th>
                <th className="py-3 px-4 text-left">Principal ID</th>
                <th
                  className="py-3 px-4 text-left cursor-pointer"
                  onClick={() => handleSort("blocksValidated")}
                >
                  <span className="flex items-center">
                    Blocks Validated {getSortIcon("blocksValidated")}
                  </span>
                </th>
                <th
                  className="py-3 px-4 text-left cursor-pointer"
                  onClick={() => handleSort("rewards")}
                >
                  <span className="flex items-center">
                    Rewards {getSortIcon("rewards")}
                  </span>
                </th>
                <th className="py-3 px-4 text-left">Last Memo</th>
              </tr>
            </thead>
            <tbody>
              {sortedValidators.map((validator, index) => (
                <tr
                  key={validator.principal}
                  className="border-b border-white/10 hover:bg-white/5"
                >
                  <td className="py-3 px-4 font-medium">{index + 1}</td>
                  <td className="py-3 px-4">
                    <div
                      className="font-medium truncate max-w-[200px]"
                      title={validator.principal}
                    >
                      {formatPrincipal(validator.principal)}
                    </div>
                  </td>
                  <td className="py-3 px-4">
                    {validator.blocksValidated.toLocaleString()}
                  </td>
                  <td className="py-3 px-4 text-blue-400">
                    {validator.rewards.toLocaleString()} DCT
                  </td>
                  <td
                    className="py-3 px-4 truncate max-w-[200px]"
                    title={validator.memo}
                  >
                    {validator.memo || "N/A"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}
