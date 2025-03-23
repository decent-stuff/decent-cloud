import { Identity } from '@dfinity/agent';
import { Principal } from '@dfinity/principal';
import { updateCanister } from './icp-utils';
import { getTokenCanisterId, fetchTokenDecimals } from './token-utils';

// Interface for token transfer parameters
export interface TokenTransferParams {
    recipient: string;
    amount: string;
    tokenType: 'ICP' | 'USDT' | 'USDC' | 'DCT';
    identity: Identity;
    principal: Principal;
}

// Stub function for sending funds
// This will be implemented separately in the future
export const sendFunds = async (params: TokenTransferParams): Promise<{ success: boolean; message: string }> => {
    try {
        const blockIndex = await sendIcrc1Transfer(
            getTokenCanisterId(params.tokenType),
            params.identity,
            [],
            { owner: Principal.fromText(params.recipient), subaccount: [] },
            params.amount,
            [],
            [],
        );
        const message = `Transfer successful, block index: ${blockIndex.toString()}`;

        return { success: true, message };
    } catch (err) {
        const error = err as Error;
        return { success: false, message: error.message || JSON.stringify(err) };
    }
};

interface NatResult {
    Ok?: bigint;
    Err?: string;
}

/**
 * Send ICRC-1 transfer
 * @param canisterId The canister ID of the ICRC-1 token
 * @param recipient The recipient's account (principal and optional subaccount)
 * @param amountString The amount to transfer (in smallest units)
 * @param identity The sender's identity
 * @param fee The fee to pay for the transfer (optional)
 * @param memo The memo to attach to the transfer (optional)
 * @param fromSubaccount The subaccount of the sender (optional)
 * @returns The block index of the transfer
 */
export async function sendIcrc1Transfer(
    canisterId: Principal,
    identity: Identity,
    fromSubaccount: Uint8Array | [] = [],
    recipient: { owner: Principal; subaccount?: [Uint8Array] | [] },
    amountString: string,
    fee: number | [] = [],
    memo: Uint8Array | [] = [],
): Promise<bigint> {
    const amountValue = parseFloat(amountString);
    const tokenDecimals = await fetchTokenDecimals(canisterId.toText(), identity);
    const decimalFactor = 10 ** tokenDecimals;

    const transferArgs = {
        to: recipient,
        amount: Math.round(amountValue * decimalFactor),
        fee,
        memo,
        from_subaccount: fromSubaccount,
        created_at_time: [],
    };

    const result = (await updateCanister('icrc1_transfer', [transferArgs], identity, { canisterId })) as NatResult;

    if (result) {
        if (result.Ok) {
            return result.Ok;
        } else if (result.Err) {
            return Promise.reject(new Error(`Failed to send tokens: ${result.Err}`));
        }
    }

    return Promise.reject(new Error('Failed to send tokens: No result returned'));
}

// Stub function for topping up funds
// This will redirect to the appropriate exchange or service
export const getTopUpUrl = (tokenType: 'ICP' | 'USDT' | 'USDC' | 'DCT'): string => {
    switch (tokenType) {
        case 'DCT':
            return 'https://www.kongswap.io/swap?from=ryjl3-tyaaa-aaaaa-aaaba-cai&to=ggi4a-wyaaa-aaaai-actqq-cai';
        case 'ICP':
            return 'https://www.kongswap.io/swap?to=ryjl3-tyaaa-aaaaa-aaaba-cai';
        case 'USDT':
            return 'https://www.kongswap.io/swap?to=cngnf-vqaaa-aaaar-qag4q-cai';
        case 'USDC':
            return 'https://www.kongswap.io/swap?to=xevnm-gaaaa-aaaar-qafnq-cai';
    }
};
