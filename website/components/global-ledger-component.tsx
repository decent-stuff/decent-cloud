"use client";

import { useEffect, useState } from "react";
import { ledgerService } from "@/lib/ledger-service";

// Default polling frequency for the global background service
const GLOBAL_POLLING_FREQUENCY = 30000; // 30 seconds (less frequent for background service)

/**
 * GlobalLedgerComponent that initializes and starts the ledger service.
 * This component should be included in the root layout to ensure the ledger service
 * is running across all pages in the application
 */
export function GlobalLedgerComponent() {
  const [retryCount, setRetryCount] = useState(0);
  const MAX_RETRIES = 5;

  useEffect(() => {
    let retryTimeout: NodeJS.Timeout | null = null;

    // Initialize and start the ledger service when the component mounts
    const initLedgerService = async () => {
      try {
        // Initialize the client
        const success = await ledgerService.initialize();
        if (!success) {
          throw new Error("Ledger service initialization returned false");
        }

        // Start polling with a longer interval for the global service
        await ledgerService.setPollingInterval(GLOBAL_POLLING_FREQUENCY);

        console.log("Global ledger service initialized and polling started");
        setRetryCount(0); // Reset retry count on success
      } catch (error) {
        console.error("Failed to initialize global ledger service:", error);

        // Implement retry logic with exponential backoff
        if (retryCount < MAX_RETRIES) {
          const nextRetryDelay = Math.min(
            1000 * Math.pow(2, retryCount),
            60000
          ); // Max 60s delay
          console.log(
            `Retrying initialization in ${nextRetryDelay}ms (attempt ${
              retryCount + 1
            }/${MAX_RETRIES})`
          );

          retryTimeout = setTimeout(() => {
            setRetryCount((prev) => prev + 1);
            void initLedgerService();
          }, nextRetryDelay);
        }
      }
    };

    void initLedgerService();

    // Clean up function to stop polling when the component unmounts
    // This should only happen when the entire app is unmounted, not on page changes
    return () => {
      if (retryTimeout) {
        clearTimeout(retryTimeout);
      }
      ledgerService.stopPolling();
      console.log("Global ledger service polling stopped");
    };
  }, [retryCount]);

  // This component doesn't render anything
  return null;
}
