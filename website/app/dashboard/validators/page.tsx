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
} from "@fortawesome/free-solid-svg-icons";
// import { ledgerService } from "@/lib/ledger-service";
import HeaderSection from "@/components/ui/header";
import { Card } from "@/components/ui/card";
// import { useAuth } from "@/lib/auth-context";
// import { useRouter } from "next/navigation";
import { ValidationResult } from "@/lib/blockchain-validator";
import { BlockchainValidator } from "@/components/blockchain-validator";

// Mock data for demonstration purposes
const mockValidators = [
  {
    id: "1",
    name: "AlphaNode",
    principal: "abcde-fghij-klmno-pqrst-uvwxy-zabcd-efghi-jklmn-opqrs-tuvwx-yz",
    blocksValidated: 1245,
    uptime: 99.98,
    rewards: 3245.75,
    stake: 50000,
    joinDate: "2024-10-15",
  },
  {
    id: "2",
    name: "BetaValidator",
    principal: "bcdef-ghijk-lmnop-qrstu-vwxyz-abcde-fghij-klmno-pqrst-uvwxy-za",
    blocksValidated: 1189,
    uptime: 99.95,
    rewards: 3102.5,
    stake: 45000,
    joinDate: "2024-10-18",
  },
  {
    id: "3",
    name: "GammaSecure",
    principal: "cdefg-hijkl-mnopq-rstuv-wxyza-bcdef-ghijk-lmnop-qrstu-vwxyz-ab",
    blocksValidated: 1302,
    uptime: 99.99,
    rewards: 3398.25,
    stake: 55000,
    joinDate: "2024-10-10",
  },
  {
    id: "4",
    name: "DeltaNode",
    principal: "defgh-ijklm-nopqr-stuvw-xyzab-cdefg-hijkl-mnopq-rstuv-wxyza-bc",
    blocksValidated: 1156,
    uptime: 99.92,
    rewards: 3015.65,
    stake: 42000,
    joinDate: "2024-10-20",
  },
  {
    id: "5",
    name: "EpsilonVal",
    principal: "efghi-jklmn-opqrs-tuvwx-yzabc-defgh-ijklm-nopqr-stuvw-xyzab-cd",
    blocksValidated: 1278,
    uptime: 99.97,
    rewards: 3335.8,
    stake: 52000,
    joinDate: "2024-10-12",
  },
];

type SortField = "blocksValidated" | "uptime" | "rewards" | "stake";
type SortDirection = "asc" | "desc";

export default function ValidatorsPage() {
  const [validators, setValidators] = useState(mockValidators);
  const [sortField, setSortField] = useState<SortField>("blocksValidated");
  const [sortDirection, setSortDirection] = useState<SortDirection>("desc");
  // const { isAuthenticated } = useAuth();
  // const router = useRouter();
  const [isLoading] = useState(true);
  const [setValidationResult] = useState<ValidationResult | null>(null);

  // Sort validators when sort parameters change
  useEffect(() => {
    const sortedValidators = [...mockValidators].sort((a, b) => {
      if (sortDirection === "asc") {
        return a[sortField] - b[sortField];
      } else {
        return b[sortField] - a[sortField];
      }
    });
    setValidators(sortedValidators);
  }, [sortField, sortDirection]);

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-white text-xl">Loading validators...</div>
      </div>
    );
  }

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
  };

  return (
    <div className="container mx-auto px-4 py-8">
      <HeaderSection title="Validation Dashboard" />

      <div className="bg-white/10 p-6 rounded-lg backdrop-blur-sm mb-6">
        {/* Blockchain Validation Section */}
        <div className="mt-8 bg-white/10 p-6 rounded-lg backdrop-blur-sm">
          <h3 className="text-xl font-semibold mb-2 text-white">
            Blockchain Validation
          </h3>
          <p className="text-white/90 mb-4">
            As a validator, you can participate in blockchain validation by
            checking in with the network. This helps maintain the integrity and
            security of the Decent Cloud network.
          </p>

          {/* Use the refactored BlockchainValidator component */}
          <BlockchainValidator
            defaultMemo="I Validated DC Ledger from the website!"
            darkMode={true}
            renderAsCard={false}
            onValidationComplete={handleValidationComplete}
          />
        </div>

        {/* Top 3 Validators */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
          {validators.slice(0, 3).map((validator, index) => (
            <div
              key={validator.id}
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
                {validator.name}
              </h4>
              <p className="text-white/70 text-sm mb-2 truncate max-w-full">
                {validator.principal.substring(0, 10)}...
                {validator.principal.substring(validator.principal.length - 5)}
              </p>
              <div className="text-blue-400 font-bold text-xl mb-2">
                {validator.blocksValidated.toLocaleString()} blocks
              </div>
              <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm w-full">
                <span className="text-white/70">Uptime:</span>
                <span className="text-white text-right">
                  {validator.uptime}%
                </span>
                <span className="text-white/70">Rewards:</span>
                <span className="text-white text-right">
                  {validator.rewards.toLocaleString()} DCT
                </span>
                <span className="text-white/70">Stake:</span>
                <span className="text-white text-right">
                  {validator.stake.toLocaleString()} DCT
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>

      <Card className="p-6 bg-white/10 backdrop-blur-sm rounded-lg border border-white/20">
        <div className="flex justify-between items-center mb-4">
          <h3 className="text-xl font-semibold text-white">All Validators</h3>
          <div className="text-xs text-white/70 bg-blue-500/20 px-3 py-1 rounded-full">
            {validators.length} validators
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-white">
            <thead>
              <tr className="border-b border-white/20">
                <th className="py-3 px-4 text-left">Rank</th>
                <th className="py-3 px-4 text-left">Name</th>
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
                  onClick={() => handleSort("uptime")}
                >
                  <span className="flex items-center">
                    Uptime {getSortIcon("uptime")}
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
                <th
                  className="py-3 px-4 text-left cursor-pointer"
                  onClick={() => handleSort("stake")}
                >
                  <span className="flex items-center">
                    Stake {getSortIcon("stake")}
                  </span>
                </th>
              </tr>
            </thead>
            <tbody>
              {validators.map((validator, index) => (
                <tr
                  key={validator.id}
                  className="border-b border-white/10 hover:bg-white/5"
                >
                  <td className="py-3 px-4 font-medium">{index + 1}</td>
                  <td className="py-3 px-4">
                    <div>
                      <div className="font-medium">{validator.name}</div>
                      <div className="text-xs text-white/70 truncate max-w-[150px]">
                        {validator.principal.substring(0, 10)}...
                        {validator.principal.substring(
                          validator.principal.length - 5
                        )}
                      </div>
                    </div>
                  </td>
                  <td className="py-3 px-4">
                    {validator.blocksValidated.toLocaleString()}
                  </td>
                  <td className="py-3 px-4">{validator.uptime}%</td>
                  <td className="py-3 px-4 text-blue-400">
                    {validator.rewards.toLocaleString()} DCT
                  </td>
                  <td className="py-3 px-4">
                    {validator.stake.toLocaleString()} DCT
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
