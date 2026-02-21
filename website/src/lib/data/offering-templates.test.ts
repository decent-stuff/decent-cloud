import { describe, it, expect } from 'vitest';
import { OFFERING_TEMPLATES } from './offering-templates';

describe('OFFERING_TEMPLATES', () => {
	it('has at least 3 templates', () => {
		expect(OFFERING_TEMPLATES.length).toBeGreaterThanOrEqual(3);
	});

	it('has no duplicate keys', () => {
		const keys = OFFERING_TEMPLATES.map((t) => t.key);
		expect(new Set(keys).size).toBe(keys.length);
	});

	it('all templates have required fields', () => {
		for (const t of OFFERING_TEMPLATES) {
			expect(t.key).toBeTruthy();
			expect(t.label).toBeTruthy();
			expect(t.offerName).toBeTruthy();
			expect(t.productType).toBeTruthy();
			expect(['public', 'private']).toContain(t.visibility);
		}
	});

	it('all prices are positive or null', () => {
		for (const t of OFFERING_TEMPLATES) {
			if (t.monthlyPrice !== null) {
				expect(t.monthlyPrice).toBeGreaterThan(0);
			}
		}
	});
});
