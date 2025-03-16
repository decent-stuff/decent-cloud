import { LedgerEntry, LedgerBlock, decentCloudLedger } from '@decent-stuff/dc-client';

// Validator information interface
export interface ValidatorInfo {
    principal: string;
    name?: string;
    blocksValidated: number;
    rewards: number;
    stake: number;
    lastValidation: number;
    memo: string;
}

// Token transfer interface
export interface TokenTransfer {
    from: string;
    to: string;
    amount: number;
    timestamp: number;
    memo: string;
}

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

    // Get the last entry parent block hash
    async getLastEntryParentBlockHash(): Promise<string | null> {
        try {
            const entries = await this.getAllEntries();
            const checkInEntries = entries.filter(entry => entry.label === 'NPCheckIn');

            if (checkInEntries.length === 0) {
                return null;
            }

            // Sort by block offset to get the latest
            checkInEntries.sort((a, b) => b.blockOffset - a.blockOffset);

            const latestCheckIn = checkInEntries[0];
            if (latestCheckIn.value && typeof latestCheckIn.value === 'object') {
                const value = latestCheckIn.value as { parent_hash?: string };
                return value.parent_hash || null;
            }

            return null;
        } catch (error) {
            console.error('Error getting last entry parent block hash:', error);
            return null;
        }
    }

    // Get all validator information
    async getValidators(): Promise<ValidatorInfo[]> {
        try {
            const entries = await this.getAllEntries();
            const blocks = await this.getAllBlocks();

            // Get all NPCheckIn entries (validators)
            const checkInEntries = entries.filter(entry => entry.label === 'NPCheckIn');

            // Map to track validators by principal
            const validatorMap = new Map<string, ValidatorInfo>();

            // Process check-in entries
            for (const entry of checkInEntries) {
                const principalWords = (entry.key as string).split(' ');
                const principal = principalWords[2] || entry.key;
                const value = entry.value as {
                    parent_hash?: string;
                    signature?: string;
                    verified?: string;
                    memo?: string;
                };

                if (!value || typeof value !== 'object') continue;

                // Find the block for this entry to get timestamp
                const block = blocks.find(b => b.blockOffset === entry.blockOffset);
                const timestamp = block ? block.timestampNs : 0;

                // Get or create validator info
                let validator = validatorMap.get(principal);
                if (!validator) {
                    validator = {
                        principal,
                        blocksValidated: 0,
                        rewards: 0,
                        stake: 0,
                        lastValidation: 0,
                        memo: ''
                    };
                    validatorMap.set(principal, validator);
                }

                // Update validator info
                validator.blocksValidated++;
                validator.lastValidation = Math.max(validator.lastValidation, timestamp);
                validator.memo = value.memo || '';

                // Find token transfer in the same block (reward)
                const blockEntries = await this.getBlockEntries(entry.blockOffset);
                const blockTransfers = blockEntries.filter(entry => entry.label === 'DCTokenTransfer');
                console.log('blockTransfers', blockTransfers);

                for (const transfer of blockTransfers) {
                    const transferValue = transfer.value as {
                        V1?: {
                            from?: { owner: string; subaccount: null | string };
                            to?: { owner: string; subaccount: null | string };
                            amount?: number | string;
                            created_at_time?: number;
                            memo?: string;
                        }
                    };
                    if (!transferValue || !transferValue.V1) continue;

                    const v1 = transferValue.V1;
                    console.log('v1', v1, 'principal', principal);
                    // Check if this is a reward transfer from the minting account to this validator
                    if (v1.from && v1.from.owner === "zjbo4-sknjf-hfisk-oi4" &&
                        v1.to && v1.to.owner === principal) {

                        // Convert to DCT - amount is in nano DCT (10^-12 DCT)
                        const amountInDCT = typeof v1.amount === 'string'
                            ? Number(BigInt(v1.amount)) / 1_000_000_000_000
                            : Number(v1.amount) / 1_000_000_000_000;

                        validator.rewards += amountInDCT;
                    }
                }
            }

            // Convert map to array and sort by blocks validated
            return Array.from(validatorMap.values())
                .sort((a, b) => b.blocksValidated - a.blocksValidated);
        } catch (error) {
            console.error('Error getting validators:', error);
            return [];
        }
    }

    // Fetch and store latest entries
    async fetchAndStoreLatestEntries(): Promise<void> {
        await decentCloudLedger.fetchLedgerBlocks();
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
