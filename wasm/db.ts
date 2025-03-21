import Dexie, { Table } from 'dexie';

// Constants
const DB_NAME = 'DecentCloudLedgerDB';

/**
 * LedgerDatabase class for managing ledger data in IndexedDB
 * Uses Dexie as the IndexedDB wrapper
 */
class LedgerDatabase extends Dexie {
    ledgerBlocks!: Table<LedgerBlock, number>;
    ledgerEntries!: Table<LedgerEntry, string>;

    // Flag to track if auto-heal was attempted
    private autoHealAttempted = false;

    // Error field to store database errors
    private _error: string | null = null;

    constructor() {
        console.info(`Initializing ${DB_NAME}...`);
        super(DB_NAME);

        // Define stores in a single version declaration
        this.version(5).stores({
            ledgerBlocks: 'blockOffset, timestampNs',
            ledgerEntries: '++id, *label, *key, *blockOffset',
        });

        // Initialize the database asynchronously
        void this.initialize();
    }

    /**
     * Utility method to handle database operations with consistent error handling
     * @param operationName Name of the operation for error messages
     * @param operation Function that performs the database operation
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
            this.setError(null);

            // Execute the operation
            return await operation();
        } catch (error) {
            console.error(`Error ${operationName}:`, error);

            // Set the error message
            if (error instanceof Error) {
                this.setError(`Failed to ${operationName}: ${error.message}`);
            } else {
                this.setError(`Failed to ${operationName}: ${String(error)}`);
            }

            // Return default value or rethrow based on whether defaultValue is provided
            if (arguments.length >= 3) {
                return defaultValue as T;
            }
            throw error;
        }
    }

    /**
     * Helper method to perform a transaction on both ledger tables
     * @param operation Function that performs operations within the transaction
     */
    private async withTransaction(operation: () => Promise<void>): Promise<void> {
        await this.transaction('rw', [this.ledgerBlocks, this.ledgerEntries], operation);
    }

    /**
     * Initialize the database asynchronously
     * This method is called from the constructor and handles async operations
     * that cannot be performed directly in the constructor
     */
    private async initialize(): Promise<void> {
        try {
            // We don't use withErrorHandling here because we need special error handling for auto-heal
            // Clear any previous error
            this.setError(null);

            // Attempt to get the last block to verify database is working
            await this.getLastBlock();
            console.info(`${DB_NAME} initialized successfully.`);
        } catch (error: unknown) {
            console.error("Error initializing database:", error);

            // Set the error message
            if (error instanceof Error) {
                this.setError(`Database initialization error: ${error.message}`);
            } else {
                this.setError(`Database initialization error: ${String(error)}`);
            }

            // Auto-heal for primary key change errors
            // Type guard to check if error is an object with name and message properties
            if (
                typeof error === 'object' &&
                error !== null &&
                'name' in error &&
                'message' in error &&
                typeof error.name === 'string' &&
                typeof error.message === 'string' &&
                error.name === "DatabaseClosedError" &&
                error.message.includes('Not yet support for changing primary key') &&
                !this.autoHealAttempted
            ) {
                console.warn('Detected primary key change error. Attempting auto-heal by deleting database...');

                // Mark that we've attempted auto-heal to prevent infinite loops
                this.autoHealAttempted = true;

                // Perform auto-heal
                await this.performAutoHeal();

                // Log that auto-heal was completed
                console.warn('Auto-heal completed. Please reload the application.');
            }

            // We don't throw here since this is an async method called from the constructor
            // The error will be logged, and the application should handle reconnection logic
        }
    }

    /**
     * Perform the auto-heal process by deleting and recreating the database
     */
    async performAutoHeal(): Promise<void> {
        await this.withErrorHandling(
            'auto-heal database',
            async () => {
                // Delete the database
                await Dexie.delete(DB_NAME);
                console.log('Database deleted successfully as part of auto-heal process.');
                // The database will be recreated on next access
            }
        );
    }

    /**
     * Get the last ledger entry (with the highest timestamp)
     * @returns The last ledger entry or null if no entries exist
     */
    async getLastBlock(): Promise<LedgerBlock | null> {
        return this.withErrorHandling(
            'get last block',
            async () => await this.ledgerBlocks.orderBy('timestampNs').last() || null,
            null
        );
    }

    /**
     * Add or update multiple ledger entries in a single transaction
     * Also updates the last entry if any of the new entries has a higher timestamp
     * @param newBlocks The ledger blocks to add
     * @param newEntries The ledger entries to add or update
     */
    async bulkAddOrUpdate(newBlocks: LedgerBlock[], newEntries: LedgerEntry[]): Promise<void> {
        if (newEntries.length === 0) return;

        await this.withErrorHandling(
            'add or update entries',
            async () => {
                await this.withTransaction(async () => {
                    // Add or update all blocks
                    await this.ledgerBlocks.bulkPut(newBlocks);
                    // Add or update all entries
                    await this.ledgerEntries.bulkPut(newEntries);
                });
            }
        );
    }

    /**
     * Get all ledger entries
     * @returns All ledger entries
     */
    async getAllEntries(): Promise<LedgerEntry[]> {
        return this.withErrorHandling(
            'get all entries',
            async () => await this.ledgerEntries.toArray(),
            []
        );
    }

    /**
     * Get all ledger blocks
     * @returns All ledger blocks
     */
    async getAllBlocks(): Promise<LedgerBlock[]> {
        return this.withErrorHandling(
            'get all blocks',
            async () => await this.ledgerBlocks.toArray(),
            []
        );
    }

    /**
     * Retrieve entries for a specific block.
     *
     * @param blockOffset The offset of the block to retrieve entries for.
     * @returns {Promise<LedgerEntry[]>} An array of ledger entries for the specified block.
     */
    async getBlockEntries(blockOffset: number): Promise<LedgerEntry[]> {
        return this.withErrorHandling(
            'get block entries',
            async () => {
                if (typeof blockOffset !== 'number') {
                    const errorMsg = `blockOffset must be a number, got (${typeof blockOffset}) ${blockOffset} instead`;
                    this.setError(errorMsg);
                    throw new Error(errorMsg);
                }
                return await this.ledgerEntries.where('blockOffset').equals(blockOffset).toArray();
            },
            []
        );
    }

    /**
     * Get a specific ledger entry by key
     * @param key The key of the entry to get
     * @returns The ledger entry or undefined if not found
     */
    async getEntry(key: string): Promise<LedgerEntry | undefined> {
        return this.withErrorHandling(
            'get entry',
            async () => await this.ledgerEntries.get(key),
            undefined
        );
    }

    /**
     * Clear all ledger entries from the database
     */
    async clearAllEntries(): Promise<void> {
        await this.withErrorHandling(
            'clear entries',
            async () => {
                await this.withTransaction(async () => {
                    await this.ledgerBlocks.clear();
                    await this.ledgerEntries.clear();
                });
            }
        );
    }

    /**
     * Explicitly delete the database and reset all data
     * This can be called manually to resolve schema issues or for troubleshooting
     * @returns Promise that resolves when the database has been deleted
     */
    async resetDatabase(): Promise<void> {
        await this.withErrorHandling(
            'reset database',
            async () => {
                // Close the current instance
                this.close();

                // Delete the database
                await Dexie.delete(DB_NAME);
                console.log('Database has been completely reset.');
            }
        );
    }

    /**
     * Get the current database error
     * @returns The current error message or null if no error
     */
    getError(): string | null {
        return this._error;
    }

    /**
     * Set the database error
     * @param error The error message to set
     */
    setError(error: string | null): void {
        if (error !== this._error) {
            if (error) {
                console.error('Database error set:', error);
            } else {
                console.info('Database error cleared');
            }
            this._error = error;
        }
    }
}

// Create and export a singleton instance of the database
export const db = new LedgerDatabase();

export interface LedgerBlock {
    blockVersion: number;
    blockSize: number;
    parentBlockHash: string;
    blockHash: string;
    blockOffset: number;
    fetchCompareBytes: string;
    fetchOffset: number;
    timestampNs: number;
}

export interface LedgerEntry {
    label: string;
    key: string;
    value: unknown;
    description: string;
    blockOffset: number;
}
