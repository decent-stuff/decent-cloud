import { LedgerEntry, LedgerBlock, decentCloudLedger } from '@decent-stuff/dc-client';

class LedgerService {
    private isInitialized = false;
    private pollingInterval: NodeJS.Timeout | null = null;
    private pollingFrequencyMs = 10000; // 10 seconds by default

    // Initialize the ledger client
    async initialize(): Promise<boolean> {
        if (this.isInitialized) return true;

        try {
            await decentCloudLedger.init();
            this.isInitialized = true;
            return true;
        } catch (error) {
            console.error('Failed to initialize Decent Cloud ledger:', error);
            return false;
        }
    }

    // Start polling for ledger updates
    async startPolling(frequencyMs?: number): Promise<void> {
        if (frequencyMs) {
            this.pollingFrequencyMs = frequencyMs;
        }

        // Clear any existing polling interval
        if (this.pollingInterval) {
            clearInterval(this.pollingInterval);
        }

        // Fetch immediately on start
        try {
            await decentCloudLedger.fetchLedgerBlocks();
        } catch (error) {
            console.error('Initial fetch failed:', error);
        }

        // Set up new polling interval
        this.pollingInterval = setInterval(async () => {
            try {
                await decentCloudLedger.fetchLedgerBlocks();
            } catch (error) {
                console.error('Polling fetch failed:', error);
            }
        }, this.pollingFrequencyMs);
    }

    // Check if polling is currently active
    isPollingActive(): boolean {
        return this.pollingInterval !== null;
    }

    // Check if the service is initialized
    getInitializationStatus(): boolean {
        return this.isInitialized;
    }

    // Stop polling for ledger updates
    stopPolling(): void {
        if (this.pollingInterval) {
            clearInterval(this.pollingInterval);
            this.pollingInterval = null;
        }
    }

    // Get all entries from the latest fetch
    async getAllEntries(): Promise<LedgerEntry[]> {
        return await decentCloudLedger.getAllEntries();
    }

    // Get all blocks from the ledger
    async getAllBlocks(): Promise<LedgerBlock[]> {
        return await decentCloudLedger.getAllBlocks();
    }

    // Get entries for a specific block
    async getBlockEntries(blockOffset: number): Promise<LedgerEntry[]> {
        return await decentCloudLedger.getBlockEntries(blockOffset);
    }

    // Get the last fetched block
    async getLastFetchedBlock(): Promise<LedgerBlock | null> {
        return await decentCloudLedger.getLastFetchedBlock();
    }

    // Get a specific entry by key from the latest block
    async getEntry(key: string): Promise<LedgerEntry | undefined> {
        const entries = await this.getAllEntries();
        return entries.find(entry => entry.key === key);
    }

    // Clear all entries
    async clearAllEntries(): Promise<void> {
        await decentCloudLedger.clearStorage();
    }

    // Disconnect the client
    disconnect(): void {
        this.stopPolling();
        this.isInitialized = false;
    }
}

// Create and export a singleton instance of the service
export const ledgerService = new LedgerService();
