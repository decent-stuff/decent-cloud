export interface ContractStatusEvent {
	contract_id: string;
	status: string;
	updated_at_ns: number | undefined;
}

export function buildContractEventsUrl(pubkey: string, apiUrl: string): string {
	return `${apiUrl}/api/v1/users/${pubkey}/contract-events`;
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
