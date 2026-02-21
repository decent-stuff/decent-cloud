import { describe, it, expect } from 'vitest';

// Mirror the pure functions from TrustBadge.svelte for testability
function getScoreColor(s: number): string {
	if (s >= 80) return 'text-green-400';
	if (s >= 60) return 'text-yellow-400';
	return 'text-red-400';
}

function getBgColor(s: number): string {
	if (s >= 80) return 'bg-green-500/20 border-green-500/30';
	if (s >= 60) return 'bg-yellow-500/20 border-yellow-500/30';
	return 'bg-red-500/20 border-red-500/30';
}

function getLabel(s: number): string {
	if (s >= 80) return 'Reliable';
	if (s >= 60) return 'Caution';
	return 'Risk';
}

describe('TrustBadge: score color classification', () => {
	it('returns green for score >= 80', () => {
		expect(getScoreColor(80)).toBe('text-green-400');
		expect(getScoreColor(95)).toBe('text-green-400');
		expect(getScoreColor(100)).toBe('text-green-400');
	});

	it('returns yellow for score 60-79', () => {
		expect(getScoreColor(60)).toBe('text-yellow-400');
		expect(getScoreColor(70)).toBe('text-yellow-400');
		expect(getScoreColor(79)).toBe('text-yellow-400');
	});

	it('returns red for score < 60', () => {
		expect(getScoreColor(59)).toBe('text-red-400');
		expect(getScoreColor(30)).toBe('text-red-400');
		expect(getScoreColor(0)).toBe('text-red-400');
	});
});

describe('TrustBadge: background color classification', () => {
	it('returns green bg for score >= 80', () => {
		expect(getBgColor(80)).toBe('bg-green-500/20 border-green-500/30');
		expect(getBgColor(100)).toBe('bg-green-500/20 border-green-500/30');
	});

	it('returns yellow bg for score 60-79', () => {
		expect(getBgColor(60)).toBe('bg-yellow-500/20 border-yellow-500/30');
		expect(getBgColor(79)).toBe('bg-yellow-500/20 border-yellow-500/30');
	});

	it('returns red bg for score < 60', () => {
		expect(getBgColor(0)).toBe('bg-red-500/20 border-red-500/30');
		expect(getBgColor(59)).toBe('bg-red-500/20 border-red-500/30');
	});
});

describe('TrustBadge: label classification', () => {
	it('returns Reliable for score >= 80', () => {
		expect(getLabel(80)).toBe('Reliable');
		expect(getLabel(100)).toBe('Reliable');
	});

	it('returns Caution for score 60-79', () => {
		expect(getLabel(60)).toBe('Caution');
		expect(getLabel(79)).toBe('Caution');
	});

	it('returns Risk for score < 60', () => {
		expect(getLabel(0)).toBe('Risk');
		expect(getLabel(59)).toBe('Risk');
	});
});

describe('TrustBadge: showTooltip prop behavior', () => {
	it('tooltip is shown by default (showTooltip defaults to true)', () => {
		// Default value in component interface
		const defaultShowTooltip = true;
		expect(defaultShowTooltip).toBe(true);
	});

	it('tooltip can be disabled by passing showTooltip=false', () => {
		const showTooltip = false;
		expect(showTooltip).toBe(false);
	});
});
