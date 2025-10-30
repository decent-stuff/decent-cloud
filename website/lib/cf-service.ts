// Cloudflare Service adapter for Decent Cloud
// Replaces direct canister calls with CF service calls

interface CFResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
  details?: string;
}

interface ResultString = { Ok: string } | { Err: string };

// CF Service endpoint - replace with actual CF worker URL when deployed
const CF_SERVICE_URL = 'http://localhost:8787'; // For local development
// const CF_SERVICE_URL = 'https://decent-cloud-api.your-subdomain.workers.dev'; // For production

/**
 * Generic function to call CF service methods
 */
async function callCFService<T = any>(
  method: string,
  args: any[] = [],
  options: RequestInit = {}
): Promise<T> {
  const url = `${CF_SERVICE_URL}/api/v1/canister/${method}`;

  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
    body: JSON.stringify({ args }),
    ...options,
  });

  if (!response.ok) {
    throw new Error(`CF service error: ${response.status} ${response.statusText}`);
  }

  const result: CFResponse<T> = await response.json();

  if (!result.success) {
    throw new Error(result.error || 'Unknown CF service error');
  }

  return result.data;
}

/**
 * Wrapper to maintain compatibility with existing canister result format
 */
function processCFResult(result: any): ResultString {
  if (result && typeof result === 'object' && ('Ok' in result || 'Err' in result)) {
    return result; // Already in canister format
  }

  // Convert CF success to canister format
  if (result && typeof result === 'string') {
    return { Ok: result };
  }

  // Handle error case
  if (result && typeof result === 'object' && result.error) {
    return { Err: result.error };
  }

  // Default case
  return { Ok: JSON.stringify(result) };
}

/**
 * CF Service implementation matching canister interface
 */
export class CFService {
  // Provider operations
  async providerRegister(pubkeyBytes: Uint8Array, cryptoSignature: Uint8Array): Promise<ResultString> {
    const result = await callCFService('provider_register_anonymous', [
      Array.from(pubkeyBytes),
      Array.from(cryptoSignature),
      null // No caller principal needed for CF service
    ]);
    return processCFResult(result);
  }

  async providerUpdateProfile(
    pubkeyBytes: Uint8Array,
    profileSerialized: Uint8Array,
    cryptoSignature: Uint8Array
  ): Promise<ResultString> {
    const result = await callCFService('provider_update_profile_anonymous', [
      Array.from(pubkeyBytes),
      Array.from(profileSerialized),
      Array.from(cryptoSignature),
      null // No caller principal needed
    ]);
    return processCFResult(result);
  }

  async providerUpdateOffering(
    pubkeyBytes: Uint8Array,
    offeringSerialized: Uint8Array,
    cryptoSignature: Uint8Array
  ): Promise<ResultString> {
    const result = await callCFService('provider_update_offering_anonymous', [
      Array.from(pubkeyBytes),
      Array.from(offeringSerialized),
      Array.from(cryptoSignature),
      null // No caller principal needed
    ]);
    return processCFResult(result);
  }

  async providerListCheckedIn(): Promise<ResultString> {
    const result = await callCFService('provider_list_checked_in');
    return processCFResult(result);
  }

  async providerGetProfileByPubkeyBytes(pubkeyBytes: Uint8Array): Promise<string | null> {
    const result = await callCFService('provider_get_profile_by_pubkey_bytes', [
      Array.from(pubkeyBytes)
    ]);
    return result;
  }

  async providerGetProfileByPrincipal(principal: string): Promise<string | null> {
    const result = await callCFService('provider_get_profile_by_principal', [principal]);
    return result;
  }

  async offeringSearch(searchQuery: string): Promise<Array<{ provider_pub_key: Uint8Array; offering_compressed: Uint8Array }>> {
    const result = await callCFService('offering_search', [searchQuery]);

    return result.map((entry: any) => ({
      provider_pub_key: new Uint8Array(entry.provider_pub_key),
      offering_compressed: new Uint8Array(entry.offering_compressed)
    }));
  }

  // Contract operations
  async contractSignRequest(
    pubkeyBytes: Uint8Array,
    contractInfoSerialized: Uint8Array,
    cryptoSignature: Uint8Array
  ): Promise<ResultString> {
    const result = await callCFService('contract_sign_request_anonymous', [
      Array.from(pubkeyBytes),
      Array.from(contractInfoSerialized),
      Array.from(cryptoSignature),
      null // No caller principal needed
    ]);
    return processCFResult(result);
  }

  async contractsListPending(pubkeyBytes: Uint8Array | null): Promise<Array<[Uint8Array, Uint8Array]>> {
    const args = pubkeyBytes ? [Array.from(pubkeyBytes)] : [null];
    const result = await callCFService('contracts_list_pending', args);

    return result.map((entry: any) => [
      new Uint8Array(entry[0]),
      new Uint8Array(entry[1])
    ]);
  }

  async contractSignReply(
    pubkeyBytes: Uint8Array,
    contractReplySerialized: Uint8Array,
    cryptoSignature: Uint8Array
  ): Promise<ResultString> {
    const result = await callCFService('contract_sign_reply_anonymous', [
      Array.from(pubkeyBytes),
      Array.from(contractReplySerialized),
      Array.from(cryptoSignature),
      null // No caller principal needed
    ]);
    return processCFResult(result);
  }

  // User operations
  async userRegister(pubkeyBytes: Uint8Array, cryptoSignature: Uint8Array): Promise<ResultString> {
    const result = await callCFService('user_register_anonymous', [
      Array.from(pubkeyBytes),
      Array.from(cryptoSignature),
      null // No caller principal needed
    ]);
    return processCFResult(result);
  }

  // Check-in operations
  async getCheckInNonce(): Promise<Uint8Array> {
    const result = await callCFService('get_check_in_nonce');
    return new Uint8Array(result);
  }

  async providerCheckIn(
    pubkeyBytes: Uint8Array,
    memo: string,
    nonceCryptoSignature: Uint8Array
  ): Promise<ResultString> {
    const result = await callCFService('provider_check_in_anonymous', [
      Array.from(pubkeyBytes),
      memo,
      Array.from(nonceCryptoSignature),
      null // No caller principal needed
    ]);
    return processCFResult(result);
  }

  // Common operations
  async getIdentityReputation(pubkeyBytes: Uint8Array): Promise<bigint> {
    const result = await callCFService('get_identity_reputation', [
      Array.from(pubkeyBytes)
    ]);
    return BigInt(result);
  }

  async getRegistrationFee(): Promise<bigint> {
    const result = await callCFService('get_registration_fee');
    return BigInt(result);
  }
}

// Export singleton instance
export const cfService = new CFService();

/**
 * Utility function to switch between canister and CF service
 */
export function getCanisterService(useCFService: boolean = false) {
  if (useCFService) {
    return cfService;
  }

  // TODO: Return existing canister service
  // This would be the existing icp-utils.ts functionality
  throw new Error('Canister service not yet implemented in this adapter');
}