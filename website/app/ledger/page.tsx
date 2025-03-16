"use client";

import { useState, useEffect } from "react";
import { ledgerService } from "@/lib/ledger-service";
import { LedgerTable } from "@/components/ledger-table";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import HeaderSection from "@/components/ui/header";
import Link from "next/link";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faArrowLeft } from "@fortawesome/free-solid-svg-icons";
import { LedgerEntry } from "@decent-stuff/dc-client";

export default function LedgerPage() {
  const [entries, setEntries] = useState<LedgerEntry[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
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

  // Clear all entries from the database
  const clearEntries = async () => {
    try {
      await ledgerService.clearAllEntries();
      setEntries([]);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to clear ledger entries"
      );
    }
  };

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
            Decent Cloud Ledger Browser
          </h3>
          <p className="text-white mb-4">
            Welcome to the Decent Cloud Ledger Browser, a tool that provides
            real-time access to blockchain data directly in your browser.
          </p>
          <p className="text-white/90 mb-4">
            This application demonstrates a complete implementation of the
            Decent Cloud ledger and client interface, creating a full replica of
            the DC ledger within your browser. The entire ledger contents are
            available for exploration and analysis without requiring server-side
            processing. This approach provides outstanding performance and
            improves robustness and user experience.
          </p>
          <p className="text-white/90 mb-4">
            The browser-based ledger can be efficiently synchronized with the
            main Decent Cloud ledger through lightweight data fetching. This
            architecture empowers developers to build sophisticated applications
            for data analysis, offering searches, and custom visualizations—all
            leveraging modern browser technologies.
          </p>
        </div>

        <div className="flex flex-wrap gap-4 mb-4 justify-end">
          <div className="w-full flex justify-end mb-2">
            <h4 className="text-white text-sm font-medium">Data Controls</h4>
          </div>
          <Button
            onClick={async () => {
              setIsLoading(true);
              try {
                const currentEntries = await ledgerService.getAllEntries();
                setEntries(currentEntries);
              } catch (err) {
                console.error("Error refreshing entries:", err);
                setError(
                  err instanceof Error
                    ? err.message
                    : "Failed to refresh entries"
                );
              } finally {
                setIsLoading(false);
              }
            }}
            disabled={isLoading}
            className="bg-blue-600 hover:bg-blue-700 text-white"
          >
            {isLoading ? "Refreshing..." : "Refresh Now"}
          </Button>

          <Button
            onClick={clearEntries}
            variant="outline"
            className="border-red-400 bg-red-500/20 text-white hover:bg-red-500/40"
          >
            Reset Local Cache
          </Button>

          <div className="w-full bg-blue-900/30 p-4 rounded-lg mb-4">
            <ul className="text-white/90 text-sm list-disc pl-5 space-y-1">
              <li>
                <span className="font-medium">
                  <b>Synchronize Ledger</b>
                </span>
                : Manually re-fetch ledger data
              </li>
              <li>
                <span className="font-medium">
                  <b>Reset Local Cache</b>
                </span>
                : Clear all locally stored ledger data (the global service will
                begin collecting data again)
              </li>
            </ul>
          </div>
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
