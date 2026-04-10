import type { SignedRequestHeaders } from '$lib/types/generated/SignedRequestHeaders';

export interface ContractStatusEvent {
	contract_id: string;
	status: string;
	updated_at_ns: number | undefined;
}

export interface PasswordResetCountEvent {
	count: number;
	contract_ids: string[];
}

export interface SshKeyRotationEvent {
	contract_id: string;
	created_at: number;
	actor: string;
	details: string | null;
}

export function buildContractEventsUrl(pubkey: string, apiUrl: string, headers?: SignedRequestHeaders): string {
	const baseUrl = `${apiUrl}/api/v1/users/${pubkey}/contract-events`;
	if (!headers) {
		return baseUrl;
	}
	const params = new URLSearchParams({
		pubkey: headers['X-Public-Key'],
		signature: headers['X-Signature'],
		timestamp: headers['X-Timestamp'],
		nonce: headers['X-Nonce']
	});
	return `${baseUrl}?${params.toString()}`;
}

export function buildPasswordResetEventsUrl(
	providerPubkey: string,
	apiUrl: string,
	headers: SignedRequestHeaders,
	isAgent: boolean = false
): string {
	const baseUrl = `${apiUrl}/api/v1/providers/${providerPubkey}/password-reset-events`;
	const pubkeyKey = isAgent ? 'agent_pubkey' : 'pubkey';
	const params = new URLSearchParams({
		[pubkeyKey]: headers['X-Public-Key'],
		signature: headers['X-Signature'],
		timestamp: headers['X-Timestamp'],
		nonce: headers['X-Nonce']
	});
	return `${baseUrl}?${params.toString()}`;
}

export function parseContractEvent(data: string): ContractStatusEvent {
	const parsed = JSON.parse(data) as unknown;
	if (
		typeof parsed !== 'object' ||
		parsed === null ||
		typeof (parsed as Record<string, unknown>).contract_id !== 'string' ||
		typeof (parsed as Record<string, unknown>).status !== 'string'
	) {
		throw new Error(`Invalid contract event payload: ${data}`);
	}
	const obj = parsed as Record<string, unknown>;
	return {
		contract_id: obj.contract_id as string,
		status: obj.status as string,
		updated_at_ns:
			typeof obj.updated_at_ns === 'number' ? (obj.updated_at_ns as number) : undefined
	};
}

export function parsePasswordResetEvent(data: string): PasswordResetCountEvent {
	const parsed = JSON.parse(data) as unknown;
	if (
		typeof parsed !== 'object' ||
		parsed === null ||
		typeof (parsed as Record<string, unknown>).count !== 'number' ||
		!Array.isArray((parsed as Record<string, unknown>).contract_ids)
	) {
		throw new Error(`Invalid password reset event payload: ${data}`);
	}
	const obj = parsed as Record<string, unknown>;
	return {
		count: obj.count as number,
		contract_ids: obj.contract_ids as string[]
	};
}

export function parseSshKeyRotationEvent(data: string): SshKeyRotationEvent {
	const parsed = JSON.parse(data) as unknown;
	if (
		typeof parsed !== 'object' ||
		parsed === null ||
		typeof (parsed as Record<string, unknown>).contract_id !== 'string' ||
		typeof (parsed as Record<string, unknown>).actor !== 'string'
	) {
		throw new Error(`Invalid SSH key rotation event payload: ${data}`);
	}
	const obj = parsed as Record<string, unknown>;
	return {
		contract_id: obj.contract_id as string,
		created_at: typeof obj.created_at === 'number' ? (obj.created_at as number) : 0,
		actor: obj.actor as string,
		details: typeof obj.details === 'string' ? (obj.details as string) : null
	};
}
