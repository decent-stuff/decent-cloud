"use client";

import { useState, useEffect } from "react";
import { ledgerService } from "@/lib/ledger-service";
import { LedgerTable } from "@/components/ledger-table";
import { Card } from "@/components/ui/card";
import HeaderSection from "@/components/ui/header";
import Link from "next/link";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faArrowLeft } from "@fortawesome/free-solid-svg-icons";
import { LedgerEntry } from "@decent-stuff/dc-client";

export default function LedgerPage() {
  const [entries, setEntries] = useState<LedgerEntry[]>([]);
  const [isLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | undefined>();

  // Load entries from the global service
  useEffect(() => {
    const loadEntries = async () => {
      try {
        const currentEntries = await ledgerService.getAllEntries();
        setEntries(currentEntries);
      } catch (err) {
        console.error("Error loading entries:", err);
        setError(err instanceof Error ? err.message : "Failed to load entries");
      }
    };

    // Initial load
    void loadEntries();

    // Set up periodic refresh
    const interval = setInterval(loadEntries, 10000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="mb-4 mt-4">
        <Link
          href="/"
          className="inline-flex items-center text-blue-400 hover:text-blue-300 transition-colors"
        >
          <FontAwesomeIcon icon={faArrowLeft} className="mr-2" />
          <span>Back to Home</span>
        </Link>
      </div>
      <HeaderSection
        title="Decent Cloud Ledger Explorer"
        subtitle="Blockchain data retrieval, visualization, and analysis"
      />

      <div className="mb-6 bg-white/10 p-6 rounded-lg backdrop-blur-sm">
        <div className="mb-6">
          <h3 className="text-xl font-semibold mb-2 text-white">
            Decent Cloud Ledger Explorer
          </h3>
          <p className="text-white mb-4">
            This is a tool that provides real-time access to the Decent Cloud
            blockchain data directly from your browser.
          </p>
          <p className="text-white/90 mb-4">
            It showcases a complete implementation of the Decent Cloud Ledger
            and client interface in the browser. The entire ledger contents are
            available for exploration and analysis without requiring server-side
            processing. This approach provides outstanding performance and
            improves robustness and user experience.
          </p>
          <p className="text-white/90 mb-4">
            The browser-based ledger can be efficiently synchronized with the
            main Decent Cloud ledger dApp through lightweight incremental data
            fetching. This architecture empowers developers to build
            sophisticated applications for data analysis, offering searches,
            custom visualizations, and other innovative features—all leveraging
            modern browser technologies.
          </p>
        </div>
      </div>

      <Card className="p-6 bg-white/10 backdrop-blur-sm rounded-lg border border-white/20">
        <div className="flex justify-between items-center mb-4">
          <h3 className="text-xl font-semibold text-white">
            Blockchain Ledger Entries
          </h3>
          <div className="text-xs text-white/70 bg-blue-500/20 px-3 py-1 rounded-full">
            {entries.length} records
          </div>
        </div>
        <LedgerTable entries={entries} isLoading={isLoading} error={error} />
      </Card>
    </div>
  );
}
