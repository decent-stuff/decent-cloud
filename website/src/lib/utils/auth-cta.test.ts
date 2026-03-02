import { describe, it, expect } from 'vitest';
import { getAuthCtaClass } from './auth-cta';

describe('getAuthCtaClass', () => {
	it('uses design-system button variants for auth hierarchy', () => {
		const google = getAuthCtaClass('google');
		const seed = getAuthCtaClass('seed');
		const back = getAuthCtaClass('back');

		expect(google).toContain('btn-secondary');
		expect(seed).toContain('btn-secondary');
		expect(back).toContain('btn-tertiary');
	});

	it('uses the same control-height token for all auth CTAs', () => {
		const google = getAuthCtaClass('google');
		const seed = getAuthCtaClass('seed');
		const back = getAuthCtaClass('back');

		expect(google).toContain('btn-control-md');
		expect(seed).toContain('btn-control-md');
		expect(back).toContain('btn-control-md');
	});
});
