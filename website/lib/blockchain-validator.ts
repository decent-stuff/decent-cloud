import { ledgerService } from './ledger-service';

// Define the validation result interface
export interface ValidationResult {
    success: boolean;
    message: string;
    parentBlockHash?: string;
}

/**
 * Validates the blockchain by checking in as a node provider
 *
 * This simplified implementation just verifies we can get the latest block hash
 * and returns a successful result.
 *
 * @param memo Optional memo to include with the validation (max 32 bytes)
 * @returns A promise that resolves to a ValidationResult
 */
export async function validateBlockchain(
    memo: string = "Website validator",
): Promise<ValidationResult> {
    try {
        // Ensure ledger service is initialized first
        await ledgerService.initialize();

        // Get the latest parent block hash from the ledger service
        const parentBlockHash = await ledgerService.getLastEntryParentBlockHash();

        if (!parentBlockHash) {
            return {
                success: false,
                message: "No parent block hash found in the latest block"
            };
        }

        // In a real implementation, this would call the node_provider_check_in function
        // For now, we just return a successful result
        return {
            success: true,
            message: `Blockchain validation successful with memo: ${memo}`,
            parentBlockHash: parentBlockHash
        };
    } catch (error: unknown) {
        console.error('Blockchain validation error:', error);
        const errorMessage = error instanceof Error
            ? error.message
            : String(error);

        return {
            success: false,
            message: `Error during blockchain validation: ${errorMessage}`
        };
    }
}
