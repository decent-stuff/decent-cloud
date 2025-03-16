"use client";

import { useState, useEffect } from "react";
import { LedgerBlock, LedgerEntry } from "@decent-stuff/dc-client";
import { ledgerService } from "@/lib/ledger-service";

interface LedgerTableProps {
  entries?: LedgerEntry[];
  isLoading?: boolean;
  error?: string;
}

interface BlockWithEntries {
  block: LedgerBlock;
  entries: LedgerEntry[];
}

export function LedgerTable({ entries, isLoading, error }: LedgerTableProps) {
  const [sortField, setSortField] = useState<keyof LedgerEntry>("key");
  const [sortDirection, setSortDirection] = useState<"asc" | "desc">("desc");
  const [blocksWithEntries, setBlocksWithEntries] = useState<
    BlockWithEntries[]
  >([]);
  const [localIsLoading, setLocalIsLoading] = useState<boolean>(
    isLoading || false
  );
  const [localError, setLocalError] = useState<string | undefined>(error);
  const [searchTerm, setSearchTerm] = useState<string>("");
  const [filterField, setFilterField] = useState<string>("all");
  // Get unique labels for dropdown (moved up to use in initial state)
  const getUniqueLabels = (entries: LedgerEntry[]) => {
    return Array.from(new Set(entries.map((entry) => entry.label || "N/A")));
  };

  // Initialize with all labels except DCTokenTransfer selected
  const [selectedLabels, setSelectedLabels] = useState<string[]>(() => {
    const allLabels = getUniqueLabels(entries || []);
    return allLabels.filter((label) => label !== "DCTokenTransfer");
  });

  // If entries are provided as props, fetch the corresponding blocks
  useEffect(() => {
    if (entries) {
      let isMounted = true;

      const fetchBlocksForEntries = async () => {
        try {
          // Get all blocks
          const allBlocks = await ledgerService.getAllBlocks();

          // Create a map of blocks by offset for quick lookup
          const blockMap = new Map(
            allBlocks.map((block) => [block.blockOffset, block])
          );

          // Group entries by blockOffset
          const entriesByBlock = entries.reduce((acc, entry) => {
            if (!acc[entry.blockOffset]) {
              acc[entry.blockOffset] = [];
            }
            acc[entry.blockOffset].push(entry);
            return acc;
          }, {} as Record<number, LedgerEntry[]>);

          // Create BlockWithEntries using actual block data
          const blocks: BlockWithEntries[] = Object.entries(entriesByBlock)
            .map(([blockOffset, blockEntries]) => {
              const block = blockMap.get(Number(blockOffset));
              if (!block) return null; // Skip if block not found
              return {
                block,
                entries: blockEntries,
              };
            })
            .filter((block): block is BlockWithEntries => block !== null);

          if (isMounted) {
            setBlocksWithEntries(blocks);

            // Update selected labels
            const allLabels = getUniqueLabels(entries);
            setSelectedLabels((prev) => {
              if (prev.length === 0) {
                return allLabels.filter((label) => label !== "DCTokenTransfer");
              }
              return prev;
            });
          }
        } catch (err) {
          if (isMounted) {
            setLocalError(
              err instanceof Error
                ? err.message
                : "Failed to fetch blocks for entries"
            );
          }
        }
      };

      fetchBlocksForEntries().catch((err) => {
        if (isMounted) {
          console.error("Error fetching blocks for entries:", err);
        }
      });

      return () => {
        isMounted = false;
      };
    }
  }, [entries]);

  // If isLoading is provided as props, use it
  useEffect(() => {
    if (isLoading !== undefined) {
      setLocalIsLoading(isLoading);
    }
  }, [isLoading]);

  // If error is provided as props, use it
  useEffect(() => {
    if (error !== undefined) {
      setLocalError(error);
    }
  }, [error]);

  // If no entries are provided, fetch blocks and their entries from the ledger service
  useEffect(() => {
    if (!entries) {
      let isMounted = true;

      const fetchBlocksAndEntries = async () => {
        try {
          if (!isMounted) return;

          // Get all blocks
          const blocks = await ledgerService.getAllBlocks();

          // For each block, get its entries and create BlockWithEntries objects
          const blocksWithEntriesData = await Promise.all(
            blocks.map(async (block) => {
              const blockEntries = await ledgerService.getBlockEntries(
                block.blockOffset
              );
              return {
                block,
                entries: blockEntries,
              };
            })
          );

          if (isMounted) {
            setBlocksWithEntries(blocksWithEntriesData);
          }
        } catch (err) {
          if (isMounted) {
            setLocalError(
              err instanceof Error
                ? err.message
                : "Failed to fetch ledger blocks and entries"
            );
          }
        } finally {
          if (isMounted) {
            setLocalIsLoading(false);
          }
        }
      };

      fetchBlocksAndEntries().catch((err) => {
        if (isMounted) {
          console.error("Error fetching ledger blocks and entries:", err);
        }
      });

      return () => {
        isMounted = false;
      };
    }
  }, [entries]);

  // Handle sorting
  const handleSort = (field: keyof LedgerEntry) => {
    if (field === sortField) {
      // Toggle sort direction if clicking the same field
      setSortDirection(sortDirection === "asc" ? "desc" : "asc");
    } else {
      // Set new sort field and default to descending (newest first)
      setSortField(field);
      setSortDirection("desc");
    }
  };

  // Handle search input change
  const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSearchTerm(e.target.value);
  };

  // Handle filter field change
  const handleFilterFieldChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setFilterField(e.target.value);
  };

  // Handle label checkbox change
  const handleLabelChange = (label: string, checked: boolean) => {
    if (label === "all") {
      // If "All Labels" is checked, select all labels
      if (checked) {
        setSelectedLabels(getUniqueLabels(allEntries));
      } else {
        // If "All Labels" is unchecked, clear selection
        setSelectedLabels([]);
      }
    } else {
      // For individual labels, add or remove from selection
      if (checked) {
        setSelectedLabels((prev) => [...prev, label]);
      } else {
        setSelectedLabels((prev) => prev.filter((l) => l !== label));
      }
    }
  };

  // Toggle all labels
  const toggleAllLabels = (checked: boolean) => {
    if (checked) {
      setSelectedLabels(getUniqueLabels(allEntries));
    } else {
      setSelectedLabels([]);
    }
  };

  // Get all entries from blocks for filtering
  const allEntries = blocksWithEntries.flatMap((block) => block.entries);

  // Get unique labels for checkboxes
  const uniqueLabels = getUniqueLabels(allEntries);

  // Filter blocks and their entries based on search term, filter field, and selected labels
  const filteredBlocksWithEntries = blocksWithEntries
    .map((blockWithEntries) => {
      const filteredEntries = blockWithEntries.entries.filter((entry) => {
        // First filter by label if any are selected
        if (
          selectedLabels.length > 0 &&
          !selectedLabels.includes(entry.label || "N/A")
        ) {
          return false;
        }

        // Then filter by search term
        if (!searchTerm) return true;

        if (filterField === "all") {
          // Search in all fields including block information
          const blockFields = {
            parentBlockHash: blockWithEntries.block.parentBlockHash,
            blockOffset: blockWithEntries.block.blockOffset,
            timestampNs: blockWithEntries.block.timestampNs,
          };
          return (
            deepSearch(entry, searchTerm) || deepSearch(blockFields, searchTerm)
          );
        } else {
          // Search in specific field
          const fieldValue =
            filterField === "key"
              ? entry.key
              : filterField === "label"
              ? entry.label
              : filterField === "description"
              ? entry.description
              : String(entry.value);
          return String(fieldValue)
            .toLowerCase()
            .includes(searchTerm.toLowerCase());
        }
      });

      return {
        block: blockWithEntries.block,
        entries: filteredEntries,
      };
    })
    .filter((block) => block.entries.length > 0); // Only keep blocks that have matching entries

  // Format value
  const formatValue = (value: unknown) => {
    if (value === null || value === undefined) return "N/A";
    if (typeof value === "object") {
      try {
        return JSON.stringify(value, null, 2);
      } catch {
        // If JSON stringification fails, convert to string directly
        return String(value);
      }
    }
    return String(value);
  };

  // Render loading state
  if (localIsLoading) {
    return (
      <div className="flex justify-center items-center p-8">
        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"></div>
      </div>
    );
  }

  // Render error state
  if (localError) {
    return (
      <div
        className="bg-red-500/20 border border-red-400 text-white px-4 py-3 rounded relative"
        role="alert"
      >
        <strong className="font-bold">Error: </strong>
        <span className="block sm:inline">{localError}</span>
      </div>
    );
  }

  // Render empty state when there are no blocks at all
  if (blocksWithEntries.length === 0) {
    return (
      <div className="text-center p-8">
        <p className="text-white/80">No ledger blocks found.</p>
      </div>
    );
  }

  // Format timestamp
  const formatTimestamp = (timestampNs: number | bigint) => {
    const timestampMs =
      typeof timestampNs === "bigint"
        ? Number(timestampNs) / 1_000_000
        : timestampNs / 1_000_000; // Convert nanoseconds to milliseconds
    return new Date(timestampMs).toLocaleString();
  };

  // Render table
  return (
    <div>
      <div className="mb-4 flex flex-wrap gap-3 items-center">
        <div className="flex items-start">
          <label className="text-white text-sm mr-2 mt-1">
            Filter by Labels:
          </label>
          <div className="bg-gray-800/50 border border-gray-700 rounded p-3 max-h-[150px] overflow-y-auto">
            <div className="flex items-center mb-2">
              <input
                type="checkbox"
                id="select-all-labels"
                checked={selectedLabels.length === uniqueLabels.length}
                onChange={(e) => toggleAllLabels(e.target.checked)}
                className="mr-2"
              />
              <label
                htmlFor="select-all-labels"
                className="text-white text-sm cursor-pointer"
              >
                Select All
              </label>
            </div>
            <div className="grid grid-cols-2 gap-2">
              {uniqueLabels.map((label, index) => (
                <div key={index} className="flex items-center">
                  <input
                    type="checkbox"
                    id={`label-${index}`}
                    checked={selectedLabels.includes(label)}
                    onChange={(e) => handleLabelChange(label, e.target.checked)}
                    className="mr-2"
                  />
                  <label
                    htmlFor={`label-${index}`}
                    className="text-white text-sm cursor-pointer truncate hover:text-clip"
                    title={label}
                  >
                    {label}
                  </label>
                </div>
              ))}
            </div>
          </div>
        </div>

        <div className="flex-1 min-w-[250px]">
          <div className="relative">
            <input
              type="text"
              placeholder="Search ledger entries..."
              value={searchTerm}
              onChange={handleSearchChange}
              className="w-full px-4 py-2 bg-gray-800/50 border border-gray-700 rounded text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 pl-10"
            />
            <div className="absolute left-3 top-2.5 text-gray-400">
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
                  d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                />
              </svg>
            </div>
          </div>
        </div>

        <div className="flex items-center">
          <label className="text-white text-sm mr-2">Search within:</label>
          <select
            value={filterField}
            onChange={handleFilterFieldChange}
            className="bg-gray-800/50 border border-gray-700 rounded text-white text-sm px-3 py-2"
          >
            <option value="all">All Fields</option>
            <option value="key">Key</option>
            <option value="label">Label</option>
            <option value="description">Description</option>
            <option value="parentBlockHash">Block Hash</option>
          </select>
        </div>

        <div className="text-white/70 text-xs">
          {filteredBlocksWithEntries.reduce(
            (sum, block) => sum + block.entries.length,
            0
          )}{" "}
          of {allEntries.length} entries in {filteredBlocksWithEntries.length}{" "}
          of {blocksWithEntries.length} blocks
        </div>
      </div>

      {filteredBlocksWithEntries.length === 0 ? (
        <div className="text-center p-8 bg-gray-800/30 rounded-lg border border-gray-700">
          <p className="text-white/80">
            No entries match your search criteria. Try adjusting your filters.
          </p>
        </div>
      ) : (
        <div className="space-y-6">
          {[...filteredBlocksWithEntries]
            .sort((a, b) => {
              const aTime =
                typeof a.block.timestampNs === "bigint"
                  ? Number(a.block.timestampNs)
                  : a.block.timestampNs;
              const bTime =
                typeof b.block.timestampNs === "bigint"
                  ? Number(b.block.timestampNs)
                  : b.block.timestampNs;
              return bTime - aTime; // Sort descending (newest first)
            })
            .map((blockWithEntries) => (
              <div
                key={blockWithEntries.block.blockOffset}
                className="bg-gray-800/30 rounded-lg border border-gray-700 overflow-hidden"
              >
                {/* Block Header */}
                <div className="bg-gray-700/30 p-4">
                  <div className="grid grid-cols-2 gap-4 text-xs">
                    <div>
                      <span className="text-gray-400">Block Offset:</span>
                      <span className="ml-2 text-white">
                        {blockWithEntries.block.blockOffset}
                      </span>
                    </div>
                    <div>
                      <span className="text-gray-400">Timestamp:</span>
                      <span className="ml-2 text-white">
                        {formatTimestamp(blockWithEntries.block.timestampNs)}
                      </span>
                    </div>
                    <div className="col-span-2">
                      <span className="text-gray-400">Parent Block Hash:</span>
                      <span className="ml-2 text-white font-mono">
                        {blockWithEntries.block.parentBlockHash}
                      </span>
                    </div>
                  </div>
                </div>

                {/* Entries Table */}
                <div className="overflow-x-auto">
                  <table className="min-w-full divide-y divide-gray-700 table-fixed text-xs">
                    <thead className="bg-gray-800/50">
                      <tr>
                        <th
                          scope="col"
                          className="px-3 py-2 text-left text-xs font-medium text-white uppercase tracking-wider cursor-pointer w-[20%]"
                          onClick={() => handleSort("key")}
                        >
                          Key
                          {sortField === "key" && (
                            <span className="ml-1 text-blue-300">
                              {sortDirection === "asc" ? "↑" : "↓"}
                            </span>
                          )}
                        </th>
                        <th
                          scope="col"
                          className="px-3 py-2 text-left text-xs font-medium text-white uppercase tracking-wider cursor-pointer w-[15%]"
                          onClick={() => handleSort("label")}
                        >
                          Label
                          {sortField === "label" && (
                            <span className="ml-1 text-blue-300">
                              {sortDirection === "asc" ? "↑" : "↓"}
                            </span>
                          )}
                        </th>
                        <th
                          scope="col"
                          className="px-3 py-2 text-left text-xs font-medium text-white uppercase tracking-wider w-[45%]"
                        >
                          Value
                        </th>
                        <th
                          scope="col"
                          className="px-3 py-2 text-left text-xs font-medium text-white uppercase tracking-wider cursor-pointer w-[20%]"
                          onClick={() => handleSort("description")}
                        >
                          Description
                          {sortField === "description" && (
                            <span className="ml-1 text-blue-300">
                              {sortDirection === "asc" ? "↑" : "↓"}
                            </span>
                          )}
                        </th>
                      </tr>
                    </thead>
                    <tbody className="bg-gray-800/30 divide-y divide-gray-700">
                      {[...blockWithEntries.entries]
                        .sort((a, b) => {
                          const aValue = String(a[sortField] || "");
                          const bValue = String(b[sortField] || "");
                          return sortDirection === "asc"
                            ? aValue.localeCompare(bValue)
                            : bValue.localeCompare(aValue);
                        })
                        .map((entry) => (
                          <tr
                            key={`${entry.blockOffset}-${entry.label}-${entry.key}`}
                            className="hover:bg-gray-700/30"
                          >
                            <td
                              className="px-3 py-2 text-xs font-medium text-white truncate hover:overflow-visible hover:z-10 hover:bg-gray-800"
                              title={entry.key}
                            >
                              {entry.key}
                            </td>
                            <td
                              className="px-3 py-2 text-xs text-blue-300 truncate hover:overflow-visible hover:z-10 hover:bg-gray-800"
                              title={entry.label || "N/A"}
                            >
                              {entry.label || "N/A"}
                            </td>
                            <td className="px-3 py-2 text-xs text-white">
                              <pre className="whitespace-pre-wrap break-words max-h-40 overflow-y-auto text-xs bg-gray-800/50 p-2 rounded border border-gray-700">
                                {formatValue(entry.value)}
                              </pre>
                            </td>
                            <td
                              className="px-3 py-2 text-xs text-white/80 truncate hover:overflow-visible hover:z-10 hover:bg-gray-800"
                              title={entry.description || "N/A"}
                            >
                              {entry.description || "N/A"}
                            </td>
                          </tr>
                        ))}
                    </tbody>
                  </table>
                </div>
              </div>
            ))}
        </div>
      )}
    </div>
  );
}

function deepSearch(obj: unknown, searchTerm: string): boolean {
  const lowerTerm = searchTerm.toLowerCase();

  // If the object is a primitive value, check it.
  if (
    typeof obj === "string" ||
    typeof obj === "number" ||
    typeof obj === "boolean"
  ) {
    return String(obj).toLowerCase().includes(lowerTerm);
  }

  // If it's an array, search through its elements.
  if (Array.isArray(obj)) {
    return obj.some((item) => deepSearch(item, searchTerm));
  }

  // If it's an object, check keys and values.
  if (obj !== null && typeof obj === "object") {
    // First, check if any key matches.
    for (const key in obj) {
      if (key.toLowerCase().includes(lowerTerm)) {
        return true;
      }
    }
    // Then, check if any value matches recursively.
    return Object.values(obj).some((val) => deepSearch(val, searchTerm));
  }

  return false;
}
