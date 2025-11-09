export type MetadataValue =
	| { Nat: bigint }
	| { Int: bigint }
	| { Text: string }
	| { Blob: Uint8Array };
export type Metadata = Array<[string, MetadataValue]>;

export function getMetadataValue(metadata: Metadata, key: string): number {
	const entry = metadata.find(([k]: [string, any]) => k === key);
	if (!entry) return 0;

	const value = entry[1];
	if ('Nat' in value) {
		const num = Number(value.Nat);
		if (key === 'ledger:current_block_rewards_e9s') return num / 1_000_000_000;
		return num;
	}
	if ('Int' in value) return Number(value.Int);
	return 0;
}
