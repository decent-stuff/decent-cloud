/**
 * Minimal structural shape the trust-data check needs. Any object with a
 * `total_contracts` number satisfies it (e.g. the full `ProviderTrustMetrics`
 * from `$lib/services/api`), so this helper stays decoupled from the service
 * layer and trivially unit-testable.
 */
export interface TrustDataShape {
	total_contracts: number;
}

/**
 * A numeric trust score is only meaningful once the provider has at least one
 * contract — the backend scoring starts at 100 and only deducts for observed
 * negative signals, so a provider with zero contracts has no behavioural track
 * record and the computed score (often ~90) reads as a dishonest "Reliable"
 * verdict. When this returns false, the UI must render an N/A + neutral
 * "Not enough data" state instead of the score + coloured verdict badge.
 */
export function hasEnoughTrustData(metrics: TrustDataShape): boolean {
	return metrics.total_contracts > 0;
}
