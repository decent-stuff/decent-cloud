import { ledgerService } from './ledger-service';
import { updateCanister } from './icp-utils';
import { ed25519Sign } from '@decent-stuff/dc-client';
import type { AuthenticatedIdentityResult } from '../lib/auth-context';

// Define the result type for canister calls
interface CanisterResult {
    Ok?: string;
    Err?: string;
    [key: string]: unknown;
}

// Define the validation result interface
export interface ValidationResult {
    success: boolean;
    message: string;
    parentBlockHash?: string;
}

// Define additional data type for validation results
type ValidationAdditionalData = {
    parentBlockHash?: string;
    [key: string]: string | undefined;
};

/**
 * Helper function to process canister result
 * @param result The result from the canister call
 * @param successPrefix Prefix for success message
 * @param errorPrefix Prefix for error message
 * @param additionalData Additional data to include in the result
 * @returns Formatted ValidationResult
 */
function processCanisterResult(
    result: unknown,
    successPrefix: string,
    errorPrefix: string,
    additionalData: ValidationAdditionalData = {}
): ValidationResult {
    if (result && typeof result === 'object' && 'Ok' in result) {
        return {
            success: true,
            message: `${successPrefix}: ${(result as CanisterResult).Ok}`,
            ...additionalData
        };
    } else if (result && typeof result === 'object' && 'Err' in result) {
        return {
            success: false,
            message: `${errorPrefix}: ${(result as { Err: string }).Err}`,
            ...additionalData
        };
    } else {
        return {
            success: false,
            message: `Unexpected response format: ${JSON.stringify(result)}`,
            ...additionalData
        };
    }
}

/**
 * Helper function to handle errors
 * @param error The error object
 * @param errorPrefix Prefix for error message
 * @param additionalData Additional data to include in the result
 * @returns Formatted ValidationResult with error details
 */
function handleError(
    error: unknown,
    errorPrefix: string,
    additionalData: ValidationAdditionalData = {}
): ValidationResult {
    console.error(`${errorPrefix}:`, error);
    const errorMessage = error instanceof Error
        ? error.message
        : String(error);

    return {
        success: false,
        message: `${errorPrefix}: ${errorMessage}`,
        ...additionalData
    };
}

/**
 * Validates the blockchain by checking in as a node provider
 *
 * This implementation makes an authenticated update call to the node_provider_check_in endpoint
 * with the latest block hash signature.
 *
 * @param memo Optional memo to include with the validation (max 32 bytes)
 * @returns A promise that resolves to a ValidationResult
 */
export async function validateBlockchain(
    memo: string = "Website validator",
    authResult: AuthenticatedIdentityResult | null
): Promise<ValidationResult> {
    try {
        // Get the latest parent block hash from the ledger service
        const lastBlockHash = await ledgerService.getLastBlockHash();

        if (!lastBlockHash) {
            return {
                success: false,
                message: "No parent block hash found in the latest block"
            };
        }
        
        if (!authResult) {
            return {
                success: false,
                message: "Authentication required. Please log in with a seed-phrase based identity."
            };
        }

        const { identity, publicKeyBytes, secretKeyRaw } = authResult;
        const dataToSign = hexToUint8Array(lastBlockHash);

        // Create a signature of the last block hash
        const signatureBytes = await ed25519Sign(secretKeyRaw, dataToSign);

        // Make the authenticated update call to node_provider_check_in
        try {
            const result = await updateCanister(
                'node_provider_check_in',
                [publicKeyBytes, memo, signatureBytes],
                identity
            );

            return processCanisterResult(
                result,
                'Ledger validation response',
                'Validation failed',
                { parentBlockHash: lastBlockHash }
            );
        } catch (error: unknown) {
            return handleError(
                error,
                'Error during blockchain validation call',
                { parentBlockHash: lastBlockHash }
            );
        }
    } catch (error: unknown) {
        return handleError(error, 'Error during blockchain validation');
    }
}

/**
 * Registers the current user as a node provider
 *
 * This implementation makes an authenticated update call to the node_provider_register endpoint
 * with a signature of the public key.
 *
 * @returns A promise that resolves to a ValidationResult
 */
export async function registerProvider(
    authResult: AuthenticatedIdentityResult | null
): Promise<ValidationResult> {
    try {
        if (!authResult) {
            return {
                success: false,
                message: "Authentication required. Please log in with a seed-phrase based identity."
            };
        }

        const { identity, publicKeyBytes, secretKeyRaw } = authResult;

        // Sign the public key with itself to prove ownership
        const signatureBytes = await ed25519Sign(secretKeyRaw, publicKeyBytes);

        // Make the authenticated update call to node_provider_register
        try {
            const result = await updateCanister(
                'node_provider_register',
                [publicKeyBytes, signatureBytes],
                identity
            );

            return processCanisterResult(
                result,
                'Registration successful',
                'Registration failed'
            );
        } catch (error: unknown) {
            return handleError(error, 'Error during provider registration');
        }
    } catch (error: unknown) {
        return handleError(error, 'Error during provider registration');
    }
}

export function hexToUint8Array(hexString: string): Uint8Array {
    if (hexString.length % 2 !== 0) {
        throw new Error("Invalid hex string");
    }

    const arrayBuffer = new Uint8Array(hexString.length / 2);

    for (let i = 0; i < hexString.length; i += 2) {
        const byteValue = parseInt(hexString.substring(i, i + 2), 16);
        arrayBuffer[i / 2] = byteValue;
    }

    return arrayBuffer;
}
