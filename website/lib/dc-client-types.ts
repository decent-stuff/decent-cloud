/**
 * Type definitions for the Decent Cloud client
 */

export interface Identity {
    /**
     * Sign data with the identity
     * @param data The data to sign
     */
    sign: (data: string) => Promise<Uint8Array>;

    /**
     * Get the public key of the identity
     */
    getPublicKey: () => Uint8Array;
}
