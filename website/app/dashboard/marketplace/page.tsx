"use client";

import { useState, useEffect } from "react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faSearch } from "@fortawesome/free-solid-svg-icons";
import HeaderSection from "@/components/ui/header";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ledgerService } from "@/lib/ledger-service";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

// Define specific types instead of using 'any'
interface OfferingData {
  metadata?: {
    name?: string;
  };
  provider?: {
    name?: string;
  };
  defaults?: {
    machine_spec?: {
      instance_types?: Array<{
        cpu?: string;
        memory?: string;
        storage?: {
          size?: string;
        };
        pricing?: {
          on_demand?: {
            hour?: number | string;
          };
        };
      }>;
    };
  };
  regions?: Array<{
    name?: string;
    description?: string;
  }>;
}

interface ProcessedOffering {
  id: string;
  name: string;
  provider: string;
  price: string;
  specs: string;
  location: string;
  rating: number;
  rawData?: OfferingData;
}

export default function MarketplacePage() {
  const [searchTerm, setSearchTerm] = useState("");
  const [offerings, setOfferings] = useState<ProcessedOffering[]>([]);
  const [filteredOfferings, setFilteredOfferings] = useState<
    ProcessedOffering[]
  >([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function fetchOfferings() {
      try {
        await ledgerService.initialize();
        const allEntries = await ledgerService.getAllEntries();

        // Filter entries with the label "ProvOffering"
        const offeringEntries = allEntries.filter(
          (entry) => entry.label === "ProvOffering"
        );

        // Create a map to track seen keys to handle duplicates
        const seenKeys = new Map<string, number>();

        // Process each offering entry
        const processedOfferings = offeringEntries.map((entry, index) => {
          // Move keyCount declaration outside the try block so it's accessible in the catch block
          const keyCount = seenKeys.get(entry.key || "") || 0;
          seenKeys.set(entry.key || "", keyCount + 1);

          try {
            // Generate a unique identifier for the offering

            // Create unique ID by combining blockOffset, key and occurrence count if needed
            const uniqueId = `${entry.blockOffset}-${entry.key || index}${
              keyCount > 0 ? `-${keyCount}` : ""
            }`;

            // Try to parse the value (which is the offering payload)
            let offeringData: OfferingData = {};

            if (typeof entry.value === "object" && entry.value !== null) {
              // Attempt to extract offering data from the payload
              const payload = entry.value as Record<string, unknown>;

              if (
                payload.V1 &&
                typeof payload.V1 === "object" &&
                payload.V1 !== null &&
                "offering_payload" in payload.V1
              ) {
                try {
                  // Try to parse the offering_payload if it's JSON
                  const offeringPayload = payload.V1.offering_payload;
                  if (
                    offeringPayload instanceof Uint8Array ||
                    Array.isArray(offeringPayload)
                  ) {
                    const payloadStr = new TextDecoder().decode(
                      new Uint8Array(offeringPayload as ArrayLike<number>)
                    );
                    offeringData = JSON.parse(payloadStr) as OfferingData;
                  }
                } catch {
                  // If parsing fails, use what we have
                  offeringData = payload.V1 as unknown as OfferingData;
                }
              } else {
                offeringData = payload as unknown as OfferingData;
              }
            }

            // Extract meaningful information from the offering
            const name =
              extractStringValue(offeringData, ["metadata", "name"]) ||
              extractStringValue(offeringData, ["provider", "name"]) ||
              `Offering ${index + 1}`;

            // Use provider name if available, otherwise use the entry key (provider ID)
            const provider =
              extractStringValue(offeringData, ["provider", "name"]) ||
              entry.key ||
              "Unknown Provider";

            // Try to extract price information
            const priceInfo = extractValue(offeringData, [
              "defaults",
              "machine_spec",
              "instance_types",
              "0", // Convert number to string for path
              "pricing",
              "on_demand",
              "hour",
            ]);
            const price = priceInfo
              ? `${(Number(priceInfo) / 1000000000000).toFixed(6)} DCT/hour`
              : "Price on request";

            // Extract specs
            const cpu =
              extractStringValue(offeringData, [
                "defaults",
                "machine_spec",
                "instance_types",
                "0", // Convert number to string for path
                "cpu",
              ]) || "";
            const memory =
              extractStringValue(offeringData, [
                "defaults",
                "machine_spec",
                "instance_types",
                "0", // Convert number to string for path
                "memory",
              ]) || "";
            const storage =
              extractStringValue(offeringData, [
                "defaults",
                "machine_spec",
                "instance_types",
                "0", // Convert number to string for path
                "storage",
                "size",
              ]) || "";
            const specs =
              [cpu, memory, storage].filter(Boolean).join(", ") ||
              "Specs not specified";

            // Extract location information
            const regions = extractValue(offeringData, ["regions"]);
            let location = "Global";
            if (Array.isArray(regions) && regions.length > 0) {
              const regionNames = regions
                .map((r) => {
                  const name = typeof r.name === "string" ? r.name : "";
                  const description =
                    typeof r.description === "string" ? r.description : "";
                  return name || description;
                })
                .filter(Boolean);
              if (regionNames.length > 0) {
                location = regionNames.join(", ");
              }
            }

            return {
              id: uniqueId, // Use our uniquely generated ID
              name,
              provider,
              price,
              specs,
              location,
              rating: 4.0 + Math.random() * 0.9, // Random rating between 4.0 and 4.9
              rawData: offeringData as OfferingData,
            };
          } catch (err) {
            console.error("Error processing offering entry:", err);
            return {
              id: `error-${entry.blockOffset}-${index}-${keyCount || 0}`, // Ensure unique ID even for error cases
              name: `Offering ${index + 1}`,
              provider: entry.key || "Unknown Provider",
              price: "Price on request",
              specs: "Details not available",
              location: "Unknown",
              rating: 4.0,
              rawData: {} as OfferingData, // Empty object with the correct type
            };
          }
        });

        // Filter out entries with empty payloads
        const validOfferings = processedOfferings.filter((offering, idx) => {
          // Check if the offering data has any meaningful content
          if (!offering.rawData) return false;

          // Is the offering data completely empty?
          const isEmpty = Object.keys(offering.rawData).length === 0;

          // Check if it has any of the essential information
          const hasName =
            offering.name && offering.name !== `Offering ${idx + 1}`;
          const hasSpecs =
            offering.specs && offering.specs !== "Specs not specified";
          const hasLocation =
            offering.location &&
            offering.location !== "Global" &&
            offering.location !== "Unknown";

          return !isEmpty || hasName || hasSpecs || hasLocation;
        });

        setOfferings(validOfferings);
        setFilteredOfferings(validOfferings);
      } catch (err) {
        console.error("Error fetching offerings:", err);
        setError(
          err instanceof Error ? err.message : "Failed to fetch offerings"
        );
      } finally {
        setIsLoading(false);
      }
    }

    // Use void operator to explicitly mark the promise as intentionally not awaited
    void fetchOfferings();
  }, []);

  // Helper function to safely extract nested values from an object
  function extractValue(obj: unknown, path: string[]): unknown {
    if (!obj) return null;

    let current: unknown = obj;
    for (const key of path) {
      if (current === null || current === undefined) return null;
      if (Array.isArray(current) && !isNaN(Number(key))) {
        const index = Number(key);
        if (index >= current.length) return null;
        current = current[index];
      } else if (
        typeof current === "object" &&
        current !== null &&
        key in (current as Record<string, unknown>)
      ) {
        current = (current as Record<string, unknown>)[key];
      } else {
        return null;
      }
    }
    return current;
  }

  // Helper function to ensure string return type
  function extractStringValue(obj: unknown, path: string[]): string | null {
    const value = extractValue(obj, path);
    if (value === null || value === undefined) return null;
    return String(value);
  }

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchTerm.trim()) {
      setFilteredOfferings(offerings);
      return;
    }

    const searchTermLower = searchTerm.toLowerCase();
    const filtered = offerings.filter(
      (offering) =>
        offering.name.toLowerCase().includes(searchTermLower) ||
        offering.provider.toLowerCase().includes(searchTermLower) ||
        offering.specs.toLowerCase().includes(searchTermLower) ||
        offering.location.toLowerCase().includes(searchTermLower)
    );
    setFilteredOfferings(filtered);
  };

  return (
    <div className="container mx-auto px-4 py-8">
      <HeaderSection
        title="Marketplace"
        subtitle="Search for available cloud offerings from decentralized providers"
      />

      <div className="bg-white/10 p-6 rounded-lg backdrop-blur-sm mb-6">
        <div className="mb-6">
          <h3 className="text-xl font-semibold mb-2 text-white">
            Find Cloud Resources
          </h3>
          <p className="text-white/90 mb-4">
            Browse through available offerings from decentralized providers on
            the Decent Cloud network.
          </p>
          {offerings.length === 0 && !isLoading && !error && (
            <div className="bg-blue-900/30 p-4 rounded-lg mb-4">
              <p className="text-yellow-300 font-medium mb-2">
                No Offerings Found
              </p>
              <p className="text-white/90 text-sm">
                There are currently no offerings registered in the ledger.
                Providers need to register their offerings to appear here.
              </p>
            </div>
          )}
          {error && (
            <div className="bg-red-900/30 p-4 rounded-lg mb-4">
              <p className="text-red-300 font-medium mb-2">
                Error Loading Offerings
              </p>
              <p className="text-white/90 text-sm">{error}</p>
            </div>
          )}
        </div>

        <form onSubmit={handleSearch} className="mb-6">
          <div className="flex flex-col md:flex-row gap-4">
            <div className="flex-grow relative">
              <input
                type="text"
                placeholder="Search by name, provider, or specifications..."
                className="w-full p-3 bg-white/20 border border-white/10 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-500"
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
              />
              <FontAwesomeIcon
                icon={faSearch}
                className="absolute right-3 top-1/2 transform -translate-y-1/2 text-white/50"
              />
            </div>
            <Button
              type="submit"
              className="bg-blue-600 hover:bg-blue-700 text-white"
            >
              Search
            </Button>
          </div>
        </form>
      </div>

      <Card className="p-6 bg-white/10 backdrop-blur-sm rounded-lg border border-white/20">
        <div className="flex justify-between items-center mb-4">
          <h3 className="text-xl font-semibold text-white">
            Available Offerings
          </h3>
          <div className="text-xs text-white/70 bg-blue-500/20 px-3 py-1 rounded-full">
            {filteredOfferings.length} results
          </div>
        </div>

        {/* We don't need this dialog since we have one for each offering */}

        {isLoading ? (
          <div className="flex justify-center items-center p-8">
            <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-400"></div>
          </div>
        ) : filteredOfferings.length === 0 ? (
          <div className="text-center p-8">
            <p className="text-white/80">
              No offerings match your search criteria. Try adjusting your
              filters.
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {filteredOfferings.map((offering) => (
              <div
                key={offering.id}
                className="border border-white/10 rounded-lg p-4 bg-white/5 hover:bg-white/10 transition-colors"
              >
                <h4 className="text-lg font-medium text-white mb-2">
                  {offering.name}
                </h4>
                <div className="flex justify-between mb-2">
                  <span className="text-white/70 text-sm">Provider:</span>
                  <span className="text-white font-medium">
                    {offering.provider}
                  </span>
                </div>
                <div className="flex justify-between mb-2">
                  <span className="text-white/70 text-sm">Price:</span>
                  <span className="text-blue-400 font-medium">
                    {offering.price}
                  </span>
                </div>
                <div className="flex justify-between mb-2">
                  <span className="text-white/70 text-sm">Specs:</span>
                  <span className="text-white text-right flex-1 ml-2">
                    {offering.specs}
                  </span>
                </div>
                <div className="flex justify-between mb-2">
                  <span className="text-white/70 text-sm">Location:</span>
                  <span className="text-white text-right flex-1 ml-2">
                    {offering.location}
                  </span>
                </div>
                <div className="flex justify-between mb-4">
                  <span className="text-white/70 text-sm">Rating:</span>
                  <span className="text-yellow-400">
                    {"★".repeat(Math.floor(offering.rating))}{" "}
                    {offering.rating.toFixed(1)}
                  </span>
                </div>
                <Dialog>
                  <DialogTrigger asChild>
                    <Button className="w-full bg-blue-600 hover:bg-blue-700 text-white">
                      View Details
                    </Button>
                  </DialogTrigger>
                  <DialogContent className="max-w-4xl max-h-[80vh] bg-slate-900 text-white border border-slate-700">
                    <DialogHeader>
                      <DialogTitle className="text-xl font-bold text-white">
                        {offering.name}
                      </DialogTitle>
                      <DialogDescription className="text-slate-400">
                        Complete JSON data for this offering
                      </DialogDescription>
                    </DialogHeader>
                    <div
                      style={{
                        maxHeight: "calc(80vh - 200px)",
                        position: "relative",
                        margin: "16px 0",
                        backgroundColor: "rgb(30, 41, 59)",
                        borderRadius: "6px",
                        padding: "16px",
                        overflowY: "scroll",
                        scrollbarWidth: "thin",
                      }}
                    >
                      <pre
                        style={{
                          whiteSpace: "pre-wrap",
                          wordBreak: "break-word",
                          fontSize: "0.875rem",
                          color: "rgba(255, 255, 255, 0.9)",
                        }}
                      >
                        {JSON.stringify(offering.rawData, null, 2)}
                      </pre>
                    </div>
                  </DialogContent>
                </Dialog>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
