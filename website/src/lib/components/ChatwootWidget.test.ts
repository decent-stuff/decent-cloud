import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';

// Mock the auth store + chatwoot API so the test only exercises the widget's
// DOM-injection gating, not real auth/identity plumbing. activeIdentity emits
// null (no signed-in user) so authenticateUser() never runs.
vi.mock('$lib/stores/auth', () => ({
	authStore: {
		activeIdentity: {
			subscribe(cb: (v: null) => void) {
				cb(null);
				return () => {};
			}
		}
	}
}));
vi.mock('$lib/services/chatwoot-api', () => ({
	getChatwootIdentity: vi.fn()
}));

// Lazy-import AFTER vi.mock so the module picks up the mocks.
const ChatwootWidget = (await import('./ChatwootWidget.svelte')).default;

/** A Chatwoot SDK <script> appended by the widget onto <head>. */
function sdkScript(): HTMLScriptElement | null {
	return document.head.querySelector<HTMLScriptElement>(
		'script[src$="/packs/js/sdk.js"]'
	);
}

afterEach(() => {
	cleanup();
	// The widget appends <script> tags directly to <head> (outside Svelte's
	// tree), so cleanup() won't remove them. Strip them between tests.
	document.head
		.querySelectorAll('script[src*="/packs/js/sdk.js"]')
		.forEach((s) => s.remove());
});

describe('ChatwootWidget: env gating (no console errors when unconfigured)', () => {
	it('injects NO sdk script when websiteToken is empty', () => {
		render(ChatwootWidget, { props: { websiteToken: '' } });
		expect(sdkScript()).toBeNull();
	});

	it('injects NO sdk script when websiteToken is set but baseUrl is missing', () => {
		// Mirrors the old hardcoded-default bug: a token with no base URL must
		// NOT fall back to a dead host. The component renders nothing.
		render(ChatwootWidget, { props: { websiteToken: 'tok_abc' } });
		expect(sdkScript()).toBeNull();
	});

	it('injects NO sdk script when baseUrl is set but websiteToken is empty', () => {
		render(ChatwootWidget, { props: { websiteToken: '', baseUrl: 'https://chat.example.org' } });
		expect(sdkScript()).toBeNull();
	});

	it('injects the sdk script at the configured baseUrl when BOTH are set', () => {
		render(ChatwootWidget, {
			props: { websiteToken: 'tok_abc', baseUrl: 'https://chat.example.org' }
		});
		const script = sdkScript();
		expect(script).not.toBeNull();
		expect(script?.getAttribute('src')).toBe('https://chat.example.org/packs/js/sdk.js');
	});
});
