import { LedgerEntry, LedgerBlock, decentCloudLedger } from '@decent-stuff/dc-client';
import { isProviderCheckInLabel } from './ledger-labels';

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
    // Database error field
    private _databaseError: string | null = null;
    private isInitialized = false;

    /**
     * Utility method to handle operations with consistent error handling
     * @param operationName Name of the operation for error messages
     * @param operation Function that performs the operation
     * @param defaultValue Optional default value to return in case of error
     * @returns Result of the operation or defaultValue in case of error
     */
    private async withErrorHandling<T>(
        operationName: string,
        operation: () => Promise<T>,
        defaultValue: T
    ): Promise<T>;
    private async withErrorHandling<T>(
        operationName: string,
        operation: () => Promise<T>
    ): Promise<T>;
    private async withErrorHandling<T>(
        operationName: string,
        operation: () => Promise<T>,
        defaultValue?: T
    ): Promise<T> {
        try {
            // Clear any previous error
            this.setDatabaseError(null);

            // Execute the operation
            return await operation();
        } catch (error) {
            console.error(`Error ${operationName}:`, error);

            // Set the database error
            if (error instanceof Error) {
                this.setDatabaseError(`Error ${operationName}: ${error.message}`);
            } else {
                this.setDatabaseError(`Error ${operationName}: ${String(error)}`);
            }

            // Return default value or rethrow based on whether defaultValue is provided
            if (arguments.length >= 3) {
                return defaultValue as T;
            }
            throw error;
        }
    }
    private pollingInterval: NodeJS.Timeout | null = null;
    private pollingFrequencyMs = 10000; // 10 seconds by default

    // Initialize the ledger client
    async initialize(): Promise<boolean> {
        if (this.isInitialized) return true;

        return this.withErrorHandling(
            'initializing ledger',
            async () => {
                await decentCloudLedger.init();
                this.isInitialized = true;
                return true;
            },
            false
        );
    }

    // Start polling for ledger updates
    async setPollingInterval(frequencyMs?: number): Promise<void> {
        if (frequencyMs) {
            this.pollingFrequencyMs = frequencyMs;
        }

        // Clear any existing polling interval
        if (this.pollingInterval) {
            clearInterval(this.pollingInterval);
        }

        // Fetch immediately on start
        await this.withErrorHandling(
            'initial fetch',
            async () => {
                await decentCloudLedger.fetchLedgerBlocks();
            },
            undefined
        );

        // Set up new polling interval
        this.pollingInterval = setInterval(async () => {
            await this.withErrorHandling(
                'polling fetch',
                async () => {
                    await decentCloudLedger.fetchLedgerBlocks();
                },
                undefined
            );
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

    /**
     * Check if there's a database error
     * @returns The database error message or null if no error
     */
    // We'll implement a simpler approach that doesn't rely on direct db access
    hasDatabaseError(): boolean {
        return this._databaseError !== null;
    }

    getDatabaseError(): string | null {
        return this._databaseError;
    }

    setDatabaseError(error: string | null): void {
        this._databaseError = error;
        if (error) {
            console.error('Database error:', error);
        }
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
        return this.withErrorHandling(
            'getting all entries',
            async () => await decentCloudLedger.getAllEntries(),
            []
        );
    }

    // Get all blocks from the ledger
    async getAllBlocks(): Promise<LedgerBlock[]> {
        return this.withErrorHandling(
            'getting all blocks',
            async () => await decentCloudLedger.getAllBlocks(),
            []
        );
    }

    // Get entries for a specific block
    async getBlockEntries(blockOffset: number): Promise<LedgerEntry[]> {
        return this.withErrorHandling(
            `getting entries for block ${blockOffset}`,
            async () => await decentCloudLedger.getBlockEntries(blockOffset),
            []
        );
    }

    // Get the last fetched block
    async getLastFetchedBlock(): Promise<LedgerBlock | null> {
        return this.withErrorHandling(
            'getting last fetched block',
            async () => await decentCloudLedger.getLastFetchedBlock(),
            null
        );
    }

    // Get the last entry parent block hash
    async getLastBlockHash(): Promise<string | null> {
        return this.withErrorHandling(
            'getting last block hash',
            async () => {
                const lastBlock = await decentCloudLedger.getLastFetchedBlock();
                return lastBlock?.blockHash || "";
            },
            null
        );
    }

    // Is a particular principal registered as a provider?
    async isProviderRegistered(principal: string): Promise<boolean> {
        return this.withErrorHandling(
            'checking if provider is registered',
            async () => {
                const isRegistered = await decentCloudLedger.isProviderRegistered(principal);
                return isRegistered || false;
            },
            false
        );
    }

    // Get the balance of a specific account
    async getAccountBalance(owner: string | null, subaccount: string | null): Promise<number> {
        return this.withErrorHandling(
            'getting account balance',
            async () => await decentCloudLedger.getAccountBalance(owner, subaccount),
            0
        );
    }

    // Get all validator information
    async getValidators(): Promise<ValidatorInfo[]> {
        return this.withErrorHandling(
            'getting validators',
            async () => {
                const entries = await this.getAllEntries();
                const blocks = await this.getAllBlocks();

                // Get all provider check-in entries (supports both current and legacy labels)
                const checkInEntries = entries.filter((entry) =>
                    isProviderCheckInLabel(entry.label)
                );

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
            },
            []
        );
    }

    // Fetch and store latest entries
    async fetchAndStoreLatestEntries(): Promise<void> {
        return this.withErrorHandling(
            'fetching latest entries',
            async () => {
                await decentCloudLedger.fetchLedgerBlocks();
            }
        );
    }

    // Clear all entries
    async clearAllEntries(): Promise<void> {
        return this.withErrorHandling(
            'clearing all entries',
            async () => {
                await decentCloudLedger.clearStorage();
            }
        );
    }

    // Disconnect the client
    disconnect(): void {
        this.stopPolling();
        this.isInitialized = false;
    }
}

// Create and export a singleton instance of the service
export const ledgerService = new LedgerService();
