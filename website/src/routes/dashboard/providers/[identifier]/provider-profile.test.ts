import { describe, it, expect } from 'vitest';

function parseJsonField<T>(field: string | undefined | null): T[] {
	if (!field) return [];
	try {
		return JSON.parse(field) as T[];
	} catch {
		return [];
	}
}

function getFeedbackColor(pct: number): string {
	if (pct >= 80) return 'text-green-400';
	if (pct >= 60) return 'text-yellow-400';
	return 'text-red-400';
}

describe('Provider Profile: parseJsonField', () => {
	it('parses valid JSON array', () => {
		expect(parseJsonField<string>('["a","b","c"]')).toEqual(['a', 'b', 'c']);
	});

	it('returns empty array for null', () => {
		expect(parseJsonField(null)).toEqual([]);
	});

	it('returns empty array for undefined', () => {
		expect(parseJsonField(undefined)).toEqual([]);
	});

	it('returns empty array for empty string', () => {
		expect(parseJsonField('')).toEqual([]);
	});

	it('returns empty array for invalid JSON', () => {
		expect(parseJsonField('not json')).toEqual([]);
	});

	it('parses JSON objects array', () => {
		const input = JSON.stringify([{ question: 'Q', answer: 'A' }]);
		expect(parseJsonField<{ question: string; answer: string }>(input)).toEqual([
			{ question: 'Q', answer: 'A' }
		]);
	});
});

describe('Provider Profile: feedback color classification', () => {
	it('green for >= 80%', () => {
		expect(getFeedbackColor(80)).toBe('text-green-400');
		expect(getFeedbackColor(100)).toBe('text-green-400');
	});

	it('yellow for 60-79%', () => {
		expect(getFeedbackColor(60)).toBe('text-yellow-400');
		expect(getFeedbackColor(79)).toBe('text-yellow-400');
	});

	it('red for < 60%', () => {
		expect(getFeedbackColor(59)).toBe('text-red-400');
		expect(getFeedbackColor(0)).toBe('text-red-400');
	});
});
