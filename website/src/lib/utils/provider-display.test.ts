import { describe, it, expect } from 'vitest';
import { providerDisplayName, type ProviderIdentity } from './provider-display';

// F7: marketplace rows + offering detail reduced providers to an auto-generated
// @handle salad (e.g. @uxprovidercggf6l) even though onboarding collects a real
// provider/company name. Prefer the provider display name; fall back to the
// @handle only when no name is set; fall back to a truncated pubkey when the
// provider has no registered account (e.g. the seeded example demos).
describe('providerDisplayName', () => {
	it('prefers the provider display name when set', () => {
		const o: ProviderIdentity = { pubkey: 'ab12cd34', provider_name: 'Acme Cloud', owner_username: 'acme123' };
		expect(providerDisplayName(o)).toBe('Acme Cloud');
	});

	it('falls back to the @handle when no provider name is set', () => {
		const o: ProviderIdentity = { pubkey: 'ab12cd34', owner_username: 'uxprovidercggf6l' };
		expect(providerDisplayName(o)).toBe('@uxprovidercggf6l');
	});

	it('falls back to a truncated pubkey when neither name nor username exists (e.g. example demos)', () => {
		const o: ProviderIdentity = { pubkey: '6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', provider_name: undefined, owner_username: undefined };
		// truncatePubkey shortens to the first 8…last 4 hex chars.
		expect(providerDisplayName(o)).not.toContain('@');
		expect(providerDisplayName(o).length).toBeLessThan(o.pubkey.length);
	});

	it('prefers provider_name even when it equals the username (no @ salad)', () => {
		// Seed data where name == username: still prefer the name form (no @ prefix).
		const o: ProviderIdentity = { pubkey: 'ab12cd34', provider_name: 'uxprovidercggf6l', owner_username: 'uxprovidercggf6l' };
		expect(providerDisplayName(o)).toBe('uxprovidercggf6l');
	});
});
