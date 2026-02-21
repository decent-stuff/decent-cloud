import { describe, it, expect } from 'vitest';

// Tests for profile completeness scoring logic (mirrors the $derived computation in support/+page.svelte)
describe('profile completeness scoring', () => {
	function computeScore(items: Array<{ done: boolean }>): number {
		return Math.round((items.filter((i) => i.done).length / items.length) * 100);
	}

	it('returns 0 for no completed items', () => {
		const items = Array(6).fill({ done: false });
		expect(computeScore(items)).toBe(0);
	});

	it('returns 100 for all completed items', () => {
		const items = Array(6).fill({ done: true });
		expect(computeScore(items)).toBe(100);
	});

	it('returns 50 for half completed', () => {
		const items = [
			{ done: true },
			{ done: true },
			{ done: true },
			{ done: false },
			{ done: false },
			{ done: false }
		];
		expect(computeScore(items)).toBe(50);
	});

	it('rounds to nearest integer', () => {
		const items = [
			{ done: true },
			{ done: true },
			{ done: false },
			{ done: false },
			{ done: false },
			{ done: false }
		];
		// 2/6 = 33.33...%
		expect(computeScore(items)).toBe(33);
	});
});
