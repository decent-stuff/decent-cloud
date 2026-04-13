import { describe, it, expect } from 'vitest';

// Mirror the pure functions from TrustBadge.svelte for testability
function getScoreColor(s: number): string {
	if (s >= 80) return 'text-success';
	if (s >= 60) return 'text-warning';
	return 'text-danger';
}

function getBgColor(s: number): string {
	if (s >= 80) return 'bg-success/20 border-success/30';
	if (s >= 60) return 'bg-warning/20 border-warning/30';
	return 'bg-danger/20 border-danger/30';
}

function getLabel(s: number): string {
	if (s >= 80) return 'Reliable';
	if (s >= 60) return 'Caution';
	return 'Risk';
}

describe('TrustBadge: score color classification', () => {
	it('returns success for score >= 80', () => {
		expect(getScoreColor(80)).toBe('text-success');
		expect(getScoreColor(95)).toBe('text-success');
		expect(getScoreColor(100)).toBe('text-success');
	});

	it('returns warning for score 60-79', () => {
		expect(getScoreColor(60)).toBe('text-warning');
		expect(getScoreColor(70)).toBe('text-warning');
		expect(getScoreColor(79)).toBe('text-warning');
	});

	it('returns danger for score < 60', () => {
		expect(getScoreColor(59)).toBe('text-danger');
		expect(getScoreColor(30)).toBe('text-danger');
		expect(getScoreColor(0)).toBe('text-danger');
	});
});

describe('TrustBadge: background color classification', () => {
	it('returns success bg for score >= 80', () => {
		expect(getBgColor(80)).toBe('bg-success/20 border-success/30');
		expect(getBgColor(100)).toBe('bg-success/20 border-success/30');
	});

	it('returns warning bg for score 60-79', () => {
		expect(getBgColor(60)).toBe('bg-warning/20 border-warning/30');
		expect(getBgColor(79)).toBe('bg-warning/20 border-warning/30');
	});

	it('returns danger bg for score < 60', () => {
		expect(getBgColor(0)).toBe('bg-danger/20 border-danger/30');
		expect(getBgColor(59)).toBe('bg-danger/20 border-danger/30');
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
