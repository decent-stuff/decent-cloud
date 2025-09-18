export interface LedgerEntry {
  id?: string;
  memo?: string;
  amount?: number;
  timestamp?: number;
  [key: string]: unknown;
}

export interface LedgerBlock {
  id?: string;
  hash?: string;
  parentHash?: string;
  timestamp?: number;
  entries?: LedgerEntry[];
  [key: string]: unknown;
}

const asyncNoop = async () => undefined;

export const decentCloudLedger = {
  init: asyncNoop,
  fetchLedgerBlocks: asyncNoop,
  getAllEntries: async () => [] as LedgerEntry[],
  getAllBlocks: async () => [] as LedgerBlock[],
  getBlockEntries: async () => [] as LedgerEntry[],
  getLastFetchedBlock: async () => null as LedgerBlock | null,
  getLastBlockHash: async () => null as string | null,
  isProviderRegistered: async () => false,
  getAccountBalance: async () => 0,
  getCurrentValidators: async () => [] as unknown[],
  getRecentTokenTransfers: async () => [] as unknown[],
  clearStorage: asyncNoop,
};

export const ed25519Sign = async (
  _secretKey: Uint8Array,
  _payload: Uint8Array
): Promise<Uint8Array> => new Uint8Array();
