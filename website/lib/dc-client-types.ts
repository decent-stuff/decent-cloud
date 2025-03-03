/**
 * Type definitions for the Decent Cloud client
 */

export interface DecentCloudClient {
    /**
     * Initialize the client
     */
    initialize: () => Promise<boolean>;

    /**
     * Get the client's identity
     */
    getIdentity: () => Identity;

    /**
     * Call a canister method
     * @param method The method name to call
     * @param args The arguments to pass to the method
     */
    callCanister: (method: string, args: any[]) => Promise<any>;

    /**
     * The ledger interface for interacting with the ledger
     */
    ledger: {
        /**
         * Get a block as JSON
         * @param blockOffset The offset of the block to get
         */
        getBlockAsJson: (blockOffset: bigint) => any;
    };
}

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
