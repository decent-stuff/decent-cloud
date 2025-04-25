import { updateCanister } from './icp-utils';
import { ed25519Sign } from '@decent-stuff/dc-client';
import type { AuthenticatedIdentityResult } from './auth-context';

// Define the validation result interface
export interface OfferingResult {
    success: boolean;
    message: string;
}

/**
 * Helper function to process canister result
 * @param result The result from the canister call
 * @param successPrefix Prefix for success message
 * @param errorPrefix Prefix for error message
 * @returns Formatted OfferingResult
 */
function processCanisterResult(
    result: unknown,
    successPrefix: string,
    errorPrefix: string,
): OfferingResult {
    if (result && typeof result === 'object' && 'Ok' in result) {
        return {
            success: true,
            message: `${successPrefix}: ${(result as { Ok: string }).Ok}`,
        };
    } else if (result && typeof result === 'object' && 'Err' in result) {
        return {
            success: false,
            message: `${errorPrefix}: ${(result as { Err: string }).Err}`,
        };
    } else {
        return {
            success: false,
            message: `Unexpected response format: ${JSON.stringify(result)}`,
        };
    }
}

/**
 * Helper function to handle errors
 * @param error The error object
 * @param errorPrefix Prefix for error message
 * @returns Formatted OfferingResult with error details
 */
function handleError(
    error: unknown,
    errorPrefix: string,
): OfferingResult {
    console.error(`${errorPrefix}:`, error);
    const errorMessage = error instanceof Error
        ? error.message
        : String(error);

    return {
        success: false,
        message: `${errorPrefix}: ${errorMessage}`,
    };
}

/**
 * Updates a provider's offering with the given JSON data
 * This implementation makes an authenticated update call to the node_provider_update_offering endpoint
 * with a cryptographically signed payload
 *
 * @param offeringJson The JSON string containing the offering data
 * @param authResult The authenticated identity result
 * @returns A promise that resolves to an OfferingResult
 */
export async function updateOffering(
    offeringJson: string,
    authResult: AuthenticatedIdentityResult | null
): Promise<OfferingResult> {
    try {
        // Validate the offering JSON
        try {
            JSON.parse(offeringJson);
        } catch (e) {
            return {
                success: false,
                message: `Invalid offering JSON: ${e instanceof Error ? e.message : String(e)}`
            };
        }

        if (!authResult) {
            return {
                success: false,
                message: "Authentication required. Please log in with a seed-phrase based identity."
            };
        }

        const { identity, publicKeyBytes, secretKeyRaw } = authResult;

        // Convert offering JSON to Uint8Array for signing
        const encoder = new TextEncoder();
        const offeringBytes = encoder.encode(offeringJson);

        // Create a signature of the offering data
        const signatureBytes = await ed25519Sign(secretKeyRaw, offeringBytes);

        // Make the authenticated update call to node_provider_update_offering
        try {
            const result = await updateCanister(
                'node_provider_update_offering',
                [publicKeyBytes, offeringBytes, signatureBytes],
                identity
            );

            return processCanisterResult(
                result,
                'Offering updated successfully',
                'Failed to update offering'
            );
        } catch (error: unknown) {
            return handleError(
                error,
                'Error during offering update'
            );
        }
    } catch (error: unknown) {
        return handleError(error, 'Error updating offering');
    }
}
