import { ledgerService } from './ledger-service';
import { updateCanister } from './icp-utils';
import { identityFromSeed } from './seed-auth';
import { ed25519Sign } from '@decent-stuff/dc-client';

// Define the validation result interface
export interface ValidationResult {
    success: boolean;
    message: string;
    parentBlockHash?: string;
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

        // Get the identity from local storage
        const storedSeedPhrase = localStorage.getItem('seed_phrase');
        if (!storedSeedPhrase) {
            return {
                success: false,
                message: "Authentication required. Please log in first."
            };
        }
        console.log('Seed phrase:', storedSeedPhrase);

        // Create identity from seed phrase
        const identity = identityFromSeed(storedSeedPhrase);
        console.log('Identity created:', identity.getPrincipal().toString());

        // Get the public key bytes from the identity
        const secretKeyRaw = identity.getKeyPair().secretKey;
        console.log('Secret key bytes:', secretKeyRaw);
        const publicKeyBytes = new Uint8Array(identity.getPublicKey().rawKey);
        console.log('Public key bytes:', publicKeyBytes);

        // Create a signature of the last block hash
        const dataToSign = hexToUint8Array(lastBlockHash);
        console.log('Data to sign:', dataToSign);
        // Cast the buffer to ArrayBuffer to satisfy TypeScript
        const signatureBytes = ed25519Sign(secretKeyRaw, dataToSign);
        console.log('Signature bytes:', signatureBytes);

        // Make the authenticated update call to node_provider_check_in
        try {
            const result = await updateCanister(
                'node_provider_check_in',
                [publicKeyBytes, memo, signatureBytes],
                identity
            );

            // Parse the result
            if (result && typeof result === 'object' && 'Ok' in result) {
                return {
                    success: true,
                    message: `Blockchain validation successful with memo: ${memo}`,
                    parentBlockHash: lastBlockHash
                };
            } else if (result && typeof result === 'object' && 'Err' in result) {
                return {
                    success: false,
                    message: `Validation failed: ${(result as { Err: string }).Err}`,
                    parentBlockHash: lastBlockHash
                };
            } else {
                return {
                    success: false,
                    message: `Unexpected response format: ${JSON.stringify(result)}`,
                    parentBlockHash: lastBlockHash
                };
            }
        } catch (error: unknown) {
            console.error('Error calling node_provider_check_in:', error);
            const errorMessage = error instanceof Error
                ? error.message
                : String(error);

            return {
                success: false,
                message: `Error during blockchain validation call: ${errorMessage}`,
                parentBlockHash: lastBlockHash
            };
        }
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
