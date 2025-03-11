"use client";

import { useState, useEffect } from "react";
import { LedgerEntry } from "@/lib/db";
import { ledgerService } from "@/lib/ledger-service";
import { LedgerTable } from "@/components/ledger-table";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import HeaderSection from "@/components/ui/header";
import Link from "next/link";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faArrowLeft } from "@fortawesome/free-solid-svg-icons";

export default function LedgerPage() {
  const [entries, setEntries] = useState<LedgerEntry[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | undefined>();
  const [isPolling, setIsPolling] = useState<boolean>(false);
  const [pollingFrequency, setPollingFrequency] = useState<number>(10000); // 10 seconds

  // Fetch ledger entries on component mount
  useEffect(() => {
    let isMounted = true;

    // Just fetch entries without initializing the service
    // since it's already running globally
    fetchEntries().catch((err) => {
      if (isMounted) {
        console.error("Error fetching entries:", err);
        setError(
          err instanceof Error ? err.message : "Failed to fetch ledger entries"
        );
      }
    });

    // Check if global service is already polling
    const checkGlobalPollingStatus = async () => {
      if (ledgerService.isPollingActive()) {
        setIsPolling(true);
      }
    };

    void checkGlobalPollingStatus();

    // Clean up on unmount (no need to stop the global service)
    return () => {
      isMounted = false;
    };
  }, []);

  // Fetch ledger entries - includes fallback initialization if the global service hasn't initialized yet
  const fetchEntries = async () => {
    try {
      setIsLoading(true);
      setError(undefined);

      // Check if the service is initialized, if not, initialize it
      // This acts as a fallback in case the global service hasn't completed initialization
      if (!ledgerService.getInitializationStatus()) {
        console.log("Ledger service not initialized yet, initializing locally");
        await ledgerService.initialize();
      }

      // Trigger a fresh fetch
      await ledgerService.fetchAndStoreLatestEntries();

      // Get all entries from the database
      const allEntries = await ledgerService.getAllEntries();
      setEntries(allEntries);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to fetch ledger entries"
      );
    } finally {
      setIsLoading(false);
    }
  };

  // Toggle local UI polling state (doesn't affect global service)
  const togglePolling = async () => {
    if (isPolling) {
      // Just update local UI state, don't stop the global service
      setIsPolling(false);
    } else {
      setIsLoading(true);
      try {
        // The global service is already running, so we just update the UI state
        // This allows the page to display entries as if auto-sync is enabled
        setIsPolling(true);
      } catch (err) {
        console.error("Error handling polling state:", err);
        setError(
          err instanceof Error ? err.message : "Failed to update polling state"
        );
      } finally {
        setIsLoading(false);
      }
    }
  };

  // Effect to update entries when polling is active
  useEffect(() => {
    let updateInterval: NodeJS.Timeout | null = null;

    if (isPolling) {
      // Set up a listener to update entries when new data is fetched
      updateInterval = setInterval(async () => {
        try {
          const allEntries = await ledgerService.getAllEntries();
          setEntries(allEntries);
        } catch (err) {
          console.error("Error updating entries during polling:", err);
        }
      }, pollingFrequency);
    }

    // Clean up the interval when polling is stopped or component unmounts
    return () => {
      if (updateInterval) {
        clearInterval(updateInterval);
      }
    };
  }, [isPolling, pollingFrequency]);

  // Handle polling frequency change for UI refresh rate only
  // Global service polling frequency remains unchanged
  const handleFrequencyChange = async (
    e: React.ChangeEvent<HTMLSelectElement>
  ) => {
    const frequency = parseInt(e.target.value, 10);
    setPollingFrequency(frequency);

    // We now only update the UI refresh rate
    // The global service continues to run at its configured frequency
    console.info("UI refresh rate updated to", frequency, "ms");
  };

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
            onClick={fetchEntries}
            disabled={isLoading}
            className="bg-blue-600 hover:bg-blue-700 text-white"
          >
            {isLoading ? "Synchronizing..." : "Synchronize Ledger"}
          </Button>

          <Button
            onClick={togglePolling}
            variant={isPolling ? "destructive" : "default"}
            className={
              isPolling
                ? "bg-red-600 hover:bg-red-700 text-white"
                : "bg-green-600 hover:bg-green-700 text-white"
            }
          >
            {isPolling ? "Hide Live Updates" : "Show Live Updates"}
          </Button>

          <div className="flex items-center bg-white/20 rounded px-3 py-2">
            <span className="mr-2 text-sm text-white">UI refresh rate:</span>
            <select
              value={pollingFrequency}
              onChange={handleFrequencyChange}
              className="border rounded p-1 bg-white/90 text-gray-800"
            >
              <option value="5000">5 seconds</option>
              <option value="10000">10 seconds</option>
              <option value="30000">30 seconds</option>
              <option value="60000">1 minute</option>
            </select>
          </div>

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
                : Manually refresh and display all available ledger data from
                the local database
              </li>
              <li>
                <span className="font-medium">
                  <b>Show Live Updates</b>
                </span>
                : Display real-time updates as they are collected by the global
                background service
              </li>
              <li>
                <span className="font-medium">
                  <b>UI Refresh Rate</b>
                </span>
                : How frequently the UI should check for and display new data
                (does not affect how often the global service polls the
                blockchain)
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
