import { truncatePubkey } from './identity';

/**
 * Minimal structural shape the provider-display helper needs. Any object with
 * these fields satisfies it (e.g. the full `Offering` from `$lib/services/api`),
 * so this helper stays decoupled from the service layer.
 */
export interface ProviderIdentity {
	pubkey: string;
	provider_name?: string;
	owner_username?: string;
}

/**
 * Human-friendly provider label for marketplace rows + offering detail.
 * Prefers the provider display name (the company/provider name collected during
 * onboarding), falls back to the @handle only when no name is set, and falls
 * back to a truncated pubkey when the provider has no registered account (e.g.
 * the seeded example demos). Stops providers being reduced to an auto-generated
 * @handle salad (F7).
 *
 * NOTE: the provider PAGE link target is separate — it stays `owner_username ||
 * pubkey` because the display name is not a resolvable identifier.
 */
export function providerDisplayName(o: ProviderIdentity): string {
	if (o.provider_name) return o.provider_name;
	if (o.owner_username) return `@${o.owner_username}`;
	return truncatePubkey(o.pubkey);
}
