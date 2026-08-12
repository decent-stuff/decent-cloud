/**
 * Shared trust-score display helpers. The single source for the trust-score
 * color/label thresholds so every surface that renders a score verdict
 * (TrustDashboard, reputation leaderboard, …) stays in sync. Do NOT duplicate
 * these thresholds inline in a component.
 */

/**
 * Minimal structural shape the trust-data check needs. Any object with an
 * optional `trust_score` satisfies it (e.g. the full `ProviderTrustMetrics`
 * from `$lib/services/api`), so this helper stays decoupled from the service
 * layer and trivially unit-testable.
 */
export interface TrustDataShape {
	trust_score?: number | null;
}

/**
 * A numeric trust score is only meaningful once the provider has at least one
 * completed contract. The backend scoring starts at 100 and only deducts for
 * observed negative signals, so a provider with zero completed contracts has
 * no behavioural track record — `get_provider_trust_metrics` now stores /
 * returns trust_score = NULL in that case (mirroring reliability_score's
 * insufficient-data pattern). Treat NULL/undefined as "no track record" and
 * have the UI render N/A + a neutral "Not enough data" verdict instead of
 * the coloured score badge.
 */
export function hasEnoughTrustData(metrics: TrustDataShape): boolean {
	return metrics.trust_score != null;
}

export type TrustTier = 'reliable' | 'caution' | 'high-risk';

/** Classify a 0-100 trust score into its display tier. */
export function getScoreTier(score: number): TrustTier {
	if (score >= 80) return 'reliable';
	if (score >= 60) return 'caution';
	return 'high-risk';
}

/** Text colour class for the numeric score, by tier. */
export function getScoreColor(score: number): string {
	const tier = getScoreTier(score);
	if (tier === 'reliable') return 'text-green-400';
	if (tier === 'caution') return 'text-yellow-400';
	return 'text-red-400';
}

/** Background + border classes for the score badge, by tier. */
export function getScoreBgColor(score: number): string {
	const tier = getScoreTier(score);
	if (tier === 'reliable') return 'bg-green-500/20 border-green-500/50';
	if (tier === 'caution') return 'bg-yellow-500/20 border-yellow-500/50';
	return 'bg-red-500/20 border-red-500/50';
}

/** Human-readable verdict label for the score, by tier. */
export function getScoreLabel(score: number): string {
	const tier = getScoreTier(score);
	if (tier === 'reliable') return 'Reliable';
	if (tier === 'caution') return 'Caution';
	return 'High Risk';
}
