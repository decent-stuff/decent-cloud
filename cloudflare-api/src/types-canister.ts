// Types matching the canister interface exactly
export type ResultString = { Ok: string } | { Err: string };
export type ResultData = { Ok: { text: string; data: Uint8Array } } | { Err: string };

export interface OfferingEntry {
  provider_pub_key: Uint8Array;
  offering_compressed: Uint8Array;
}

export interface ContractEntry {
  contract_id: Uint8Array;
  contract_data: Uint8Array;
}

// Canister method signatures
export interface CanisterMethods {
  // Provider operations
  provider_register(pubkey_bytes: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString>;
  provider_update_profile(pubkey_bytes: Uint8Array, profile_serialized: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString>;
  provider_update_offering(pubkey_bytes: Uint8Array, offering_serialized: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString>;
  provider_list_checked_in(): Promise<ResultString>;
  provider_get_profile_by_pubkey_bytes(pubkey_bytes: Uint8Array): Promise<string | null>;
  provider_get_profile_by_principal(principal: string): Promise<string | null>;
  offering_search(search_query: string): Promise<OfferingEntry[]>;

  // Contract operations
  contract_sign_request(pubkey_bytes: Uint8Array, contract_info_serialized: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString>;
  contracts_list_pending(pubkey_bytes: Uint8Array | null): Promise<ContractEntry[]>;
  contract_sign_reply(pubkey_bytes: Uint8Array, contract_reply_serialized: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString>;

  // User operations
  user_register(pubkey_bytes: Uint8Array, crypto_sig: Uint8Array): Promise<ResultString>;

  // Check-in operations
  get_check_in_nonce(): Promise<Uint8Array>;
  provider_check_in(pubkey_bytes: Uint8Array, memo: string, nonce_crypto_sig: Uint8Array): Promise<ResultString>;

  // Common operations
  get_identity_reputation(pubkey_bytes: Uint8Array): Promise<bigint>;
  get_registration_fee(): Promise<bigint>;
}