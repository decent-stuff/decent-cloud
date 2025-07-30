import { Actor, HttpAgent, Identity } from '@dfinity/agent';
import { Principal } from '@dfinity/principal';
import { IDL } from '@dfinity/candid';

// Token canister IDs
const TOKEN_CANISTERS = {
    ICP: 'ryjl3-tyaaa-aaaaa-aaaba-cai',
    USDT: 'cngnf-vqaaa-aaaar-qag4q-cai',
    USDC: 'xevnm-gaaaa-aaaar-qafnq-cai',
    DCT: 'ggi4a-wyaaa-aaaai-actqq-cai'
} as const;

export function getTokenCanisterId(tokenType: 'ICP' | 'USDT' | 'USDC' | 'DCT'): Principal {
    switch (tokenType) {
        case 'ICP':
            return Principal.fromText(TOKEN_CANISTERS.ICP);
        case 'USDT':
            return Principal.fromText(TOKEN_CANISTERS.USDT);
        case 'USDC':
            return Principal.fromText(TOKEN_CANISTERS.USDC);
        case 'DCT':
            return Principal.fromText(TOKEN_CANISTERS.DCT);
    }
}

const tokenDecimals: { [canisterId: string]: number } = {};

// Create the Candid IDL factory for ICRC-1
const icrc1Interface: IDL.InterfaceFactory = ({ IDL }) => {
    const Account = IDL.Record({
        owner: IDL.Principal,
        subaccount: IDL.Opt(IDL.Vec(IDL.Nat8))
    });

    return IDL.Service({
        icrc1_balance_of: IDL.Func([Account], [IDL.Nat], ['query']),
        icrc1_decimals: IDL.Func([], [IDL.Nat8], ['query'])
    });
};

// Create an actor for interacting with ICRC-1 tokens
interface ICRC1Actor {
    icrc1_balance_of: (account: { owner: Principal; subaccount: [] }) => Promise<bigint>;
    icrc1_decimals: () => Promise<number>;
}

const createTokenActor = async (canisterId: string, identity: Identity): Promise<ICRC1Actor> => {
    const agent = await HttpAgent.create({
        host: 'https://icp-api.io',
        identity
    });

    return Actor.createActor(icrc1Interface, {
        agent,
        canisterId
    });
};

// Convert e8s (8 decimal places) to human readable format
const fromDecimals = (amount: bigint | number, decimals: number): number => {
    const divisor = 10 ** decimals;
    if (typeof amount === 'bigint') {
        return Number(amount) / divisor;
    }
    return amount / divisor;
};

// Fetch balance for a specific token
const fetchTokenBalance = async (
    canisterId: string,
    identity: Identity,
    principal: Principal
): Promise<number> => {
    try {
        const actor = await createTokenActor(canisterId, identity);
        const numDecimals = await fetchTokenDecimals(canisterId, identity);
        const balance = await actor.icrc1_balance_of({
            owner: principal,
            subaccount: []
        });

        // Convert balance to number before passing to fromE8s
        const balanceNum = typeof balance === 'bigint' ? balance : BigInt(String(balance));
        const tokenBalance = fromDecimals(balanceNum, numDecimals);
        return tokenBalance;
    } catch (error) {
        console.error(`Error fetching balance for canister ${canisterId}:`, error);
        return 0;
    }
};

// Fetch number of decimals for a specific token
export async function fetchTokenDecimals(canisterId: string, identity: Identity): Promise<number> {
    try {
        const actor = await createTokenActor(canisterId, identity);

        let numDecimals = tokenDecimals[canisterId];
        if (numDecimals === undefined) {
            numDecimals = (await actor.icrc1_decimals()) as number;
            tokenDecimals[canisterId] = numDecimals;
        }
        return numDecimals || 8;
    } catch (error) {
        console.error(`Error fetching decimals for canister ${canisterId}:`, error);
        return 8;
    }
};

// Fetch all token balances for a user
export const fetchUserBalances = async (
    identity: Identity,
    principal: Principal
): Promise<{
    icp: number;
    ckUsdt: number;
    ckUsdc: number;
    dct: number;
}> => {
    const [icp, ckUsdt, ckUsdc, dct] = await Promise.all([
        fetchTokenBalance(TOKEN_CANISTERS.ICP, identity, principal),
        fetchTokenBalance(TOKEN_CANISTERS.USDT, identity, principal),
        fetchTokenBalance(TOKEN_CANISTERS.USDC, identity, principal),
        fetchTokenBalance(TOKEN_CANISTERS.DCT, identity, principal)
    ]);

    return {
        icp,
        ckUsdt,
        ckUsdc,
        dct
    };
};

export interface Token {
    metrics?: {
        price?: number;
        volume_24h?: number;
        total_supply?: number;
        market_cap?: number;
        tvl?: number;
        updated_at?: string;
        price_change_24h?: number | null;
    };
    logo_url?: string;
    address?: string;
    fee?: number;
    fee_fixed?: string;
    token?: string;
    token_type?: string;
    chain?: string;
    pool_symbol?: string;
    pools: Array<{
        id: string;
        symbol: string;
    }>;
}

export const fetchDctPrice = async (): Promise<number> => {
    try {
        const tokens = await fetchTokensByCanisterId([TOKEN_CANISTERS.DCT]);
        return tokens[0]?.metrics?.price ?? 0;
    } catch (error) {
        console.error('Error fetching DCT price:', error);
        return 0;
    }
};

interface TokenResponse {
    items: KongSwapTokenDataRaw[];
    total_pages: number;
    total_count: number;
    page: number;
    limit: number;
}

const fetchTokenPage = async (canisterIds: string[], page: number): Promise<TokenResponse> => {
    const response = await fetch("https://api.kongswap.io/api/tokens/by_canister", {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            canister_ids: canisterIds,
            page,
            limit: 100 // Maximum limit to minimize number of requests
        })
    });

    if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
    }

    const data = await response.json() as TokenResponse;

    if (!data.items || !Array.isArray(data.items)) {
        throw new Error('Unexpected API response format');
    }

    return data;
};

export const fetchTokensByCanisterId = async (canisterIds: string[]): Promise<Token[]> => {
    try {
        // Fetch first page to get total pages
        const firstPage = await fetchTokenPage(canisterIds, 1);
        let allItems = [...firstPage.items];

        // Fetch remaining pages if any
        const remainingPages = Array.from(
            { length: firstPage.total_pages - 1 },
            (_, i) => i + 2
        );

        if (remainingPages.length > 0) {
            const remainingResults = await Promise.all(
                remainingPages.map(page => fetchTokenPage(canisterIds, page))
            );

            // Combine all items
            remainingResults.forEach(result => {
                allItems = allItems.concat(result.items);
            });
        }

        return allItems.map(parseTokenData);
    } catch (error) {
        console.error('Error fetching tokens:', error);
        return [];
    }
};

// Example response
// {
//     "items": [
//       {
//         "address": null,
//         "canister_id": "ggi4a-wyaaa-aaaai-actqq-cai",
//         "decimals": 9,
//         "fee": 0.001,
//         "fee_fixed": "1000000",
//         "has_custom_logo": false,
//         "icrc1": true,
//         "icrc2": false,
//         "icrc3": false,
//         "is_removed": false,
//         "logo_updated_at": "2025-01-31T08:58:31.720571",
//         "logo_url": "https://apibucket.nyc3.digitaloceanspaces.com/token_logos/258-logo.svg",
//         "metrics": {
//           "market_cap": "26986890.00000000",
//           "previous_price": "1.3055444",
//           "price": "0.24427591",
//           "price_change_24h": null,
//           "token_id": 258,
//           "total_supply": "21000000000000000",
//           "tvl": "459.7068663748780912",
//           "updated_at": "2025-01-31T09:06:41.888646",
//           "volume_24h": "66.533782483371756"
//         },
//         "name": "Decent Cloud",
//         "symbol": "DC",
//         "token_id": 258,
//         "token_type": "Ic"
//       }
//     ],
//     "total_pages": 1,
//     "total_count": 1,
//     "page": 1,
//     "limit": 100
//   }



interface KongSwapTokenDataRaw {
    address?: string | null;
    canister_id?: string;
    decimals?: number;
    fee?: number;
    fee_fixed?: string;
    has_custom_logo?: boolean;
    icrc1?: boolean;
    icrc2?: boolean;
    icrc3?: boolean;
    is_removed?: boolean;
    logo_updated_at?: string;
    logo_url?: string;
    metrics?: {
        market_cap?: number | string;
        previous_price?: number | string;
        price?: number | string;
        price_change_24h?: number | string | null;
        token_id?: number;
        total_supply?: number | string;
        tvl?: number | string;
        updated_at?: string;
        volume_24h?: number | string;
    };
    name?: string;
    symbol?: string;
    token_id?: number;
    token_type?: string;
    chain?: string;
    pool_symbol?: string;
}

const parseTokenData = (token: KongSwapTokenDataRaw): Token => {
    const parseMetricValue = (value: number | string | undefined | null): number => {
        if (!value) return 0;
        if (typeof value === 'number') return value;
        if (typeof value === 'string') return parseFloat(value.replaceAll(",", "")) || 0;
        return 0;
    }

    const parsePriceChange = (value: number | string | null | undefined): number | null => {
        if (value === null || value === undefined) return null;
        if (typeof value === 'number') return value;
        if (typeof value === 'string') return parseFloat(value.replaceAll(",", "")) || 0;
        return null;
    }

    // Handle logo URL based on has_custom_logo flag
    const getLogo = (logoUrl: string | undefined): string => {
        if (!logoUrl) return "";
        if (logoUrl.startsWith('http')) return logoUrl;
        return `https://api.kongswap.io${logoUrl.startsWith('/') ? '' : '/'}${logoUrl}`;
    }

    return {
        metrics: {
            price: parseMetricValue(token?.metrics?.price),
            volume_24h: parseMetricValue(token?.metrics?.volume_24h),
            total_supply: parseMetricValue(token?.metrics?.total_supply),
            market_cap: parseMetricValue(token?.metrics?.market_cap),
            tvl: parseMetricValue(token?.metrics?.tvl),
            updated_at: token?.metrics?.updated_at,
            price_change_24h: parsePriceChange(token?.metrics?.price_change_24h)
        },
        logo_url: getLogo(token?.logo_url),
        address: token.address || token.canister_id || "",
        fee: Number(token.fee || 0),
        fee_fixed: BigInt((token?.fee_fixed && typeof token.fee_fixed === 'string' ? token.fee_fixed.replaceAll("_", "") : "0") || "0").toString(),
        token: token.token_type || '',
        token_type: token.token_type || '',
        chain: token.token_type?.toUpperCase() === 'IC' ? 'ICP' : token.chain || '',
        pool_symbol: token.pool_symbol || "Pool not found",
        pools: [],
    };
};
