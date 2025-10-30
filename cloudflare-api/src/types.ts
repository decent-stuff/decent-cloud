export interface Env {
  DB: D1Database;
  TEST_DB?: D1Database;
  ENVIRONMENT: string;
  CANISTER_ID: string;
  FLUSH_INTERVAL_SECONDS: string;
  MAX_RETRY_ATTEMPTS: string;
  TEST_DB_NAME?: string;
}

export interface Script {
  id: string;
  title: string;
  description: string;
  category: string;
  tags?: string[];
  luaSource: string;
  authorName: string;
  authorId: string;
  authorPrincipal?: string; // ICP principal of the script author
  authorPublicKey?: string; // Public key for signature verification
  uploadSignature?: string; // Signature of the initial upload payload
  canisterIds?: string[];
  iconUrl?: string;
  screenshots?: string[];
  version: string;
  compatibility?: string;
  price: number;
  isPublic: boolean;
  downloads: number;
  rating: number;
  reviewCount: number;
  createdAt: string;
  updatedAt: string;
  author?: Author;
  reviews?: Review[];
}

export interface Author {
  id: string;
  username: string;
  displayName: string;
  avatar?: string;
  isVerifiedDeveloper: boolean;
}

export interface User {
  id: string;
  email?: string;
  name: string;
  isVerifiedDeveloper: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface Review {
  id: string;
  scriptId: string;
  userId: string;
  rating: number;
  comment?: string;
  createdAt: string;
  updatedAt: string;
}

export interface Purchase {
  id: string;
  scriptId: string;
  userId: string;
  price: number;
  purchaseDate: string;
}

export interface ApiResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
  details?: string;
}

export interface PaginatedResponse<T> extends ApiResponse<T> {
  total?: number;
  hasMore?: boolean;
}

// Decent Cloud Core Types
export interface DcUser {
  id: string;
  pubkey: string;
  principal?: string;
  reputation: number;
  balanceTokens: number;
  createdAt: string;
  updatedAt: string;
}

export interface ProviderProfile {
  pubkey: string;
  profileData: ArrayBuffer | Uint8Array; // Serialized profile data
  signature?: ArrayBuffer | Uint8Array;
  version: number;
  createdAt: string;
  updatedAt: string;
}

export interface ProviderOffering {
  id: string;
  providerPubkey: string;
  offeringData: ArrayBuffer | Uint8Array; // Serialized offering data
  signature?: ArrayBuffer | Uint8Array;
  version: number;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface LedgerEntry {
  id: number;
  label: string;
  key: string;
  value: ArrayBuffer;
  blockOffset: number;
  operation: 'INSERT' | 'UPDATE' | 'DELETE';
  migratedAt: string;
  icpTimestamp: number;
}

export interface ReputationChange {
  id: number;
  targetPubkey: string;
  changeAmount: number;
  reason?: string;
  blockOffset: number;
  changedAt: string;
  icpTimestamp: number;
}

export interface TokenTransfer {
  id: number;
  fromPubkey: string;
  toPubkey: string;
  amount: number;
  memo?: string;
  blockOffset: number;
  transferredAt: string;
  icpTimestamp: number;
}

export interface ContractSignature {
  id: number;
  contractId: string;
  requesterPubkey: string;
  providerPubkey: string;
  contractData: ArrayBuffer;
  signature?: ArrayBuffer;
  status: 'pending' | 'signed' | 'rejected' | 'expired';
  blockOffset: number;
  createdAt: string;
  updatedAt: string;
  icpTimestamp: number;
}

export interface SyncStatus {
  tableName: string;
  lastSyncedBlockOffset: number;
  lastSyncedAt: string;
  totalRecordsSynced: number;
  syncErrors: number;
  lastError?: string;
}

// Sync request/response types
export interface SyncRequest {
  tableName: string;
  fromBlockOffset?: number;
  batchSize?: number;
}

export interface SyncResponse {
  tableName: string;
  recordsProcessed: number;
  lastBlockOffset: number;
  errors: string[];
  hasMore: boolean;
}

// Provider profile with reputation (combined query result)
export interface ProviderProfileWithReputation {
  pubkey: string;
  profileData: ArrayBuffer | Uint8Array;
  signature?: ArrayBuffer | Uint8Array;
  version: number;
  reputation: number;
  createdAt: string;
  updatedAt: string;
}

// Canister result type for operations
export type ResultString = { Ok: string } | { Err: string };