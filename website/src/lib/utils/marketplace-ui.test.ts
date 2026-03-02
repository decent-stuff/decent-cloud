import { describe, it, expect } from 'vitest';
import {
	buildQuickPillClass,
	buildRowActionButtonClass,
	type QuickPillKind,
} from './marketplace-ui';

describe('buildQuickPillClass', () => {
	it('uses a distinct base style for filter pills', () => {
		const cls = buildQuickPillClass('filter', false, 'neutral');
		expect(cls).toContain('quick-pill-filter');
		expect(cls).not.toContain('quick-pill-preset');
	});

	it('uses a distinct base style for preset pills', () => {
		const cls = buildQuickPillClass('preset', false, 'neutral');
		expect(cls).toContain('quick-pill-preset');
		expect(cls).not.toContain('quick-pill-filter');
	});

	it('applies active color treatment by color key', () => {
		const cls = buildQuickPillClass('preset', true, 'sky');
		expect(cls).toContain('bg-sky-500/20');
		expect(cls).toContain('border-sky-500/50');
		expect(cls).toContain('text-sky-300');
	});

	it('falls back to neutral active style for unknown color key', () => {
		const cls = buildQuickPillClass('filter' as QuickPillKind, true, 'unknown');
		expect(cls).toContain('bg-primary-500/20');
		expect(cls).toContain('text-primary-300');
		expect(cls).toContain('border-primary-500/50');
	});
});

describe('buildRowActionButtonClass', () => {
	it('enforces a consistent compact height for row actions', () => {
		const rent = buildRowActionButtonClass('rent');
		const save = buildRowActionButtonClass('save', false);
		const compare = buildRowActionButtonClass('compare', false);

		expect(rent).toContain('h-7');
		expect(save).toContain('h-7');
		expect(compare).toContain('h-7');
	});

	it('builds selected state for save button', () => {
		const cls = buildRowActionButtonClass('save', true);
		expect(cls).toContain('bg-primary-500/20');
		expect(cls).toContain('border-primary-400/50');
	});

	it('builds selected state for compare button', () => {
		const cls = buildRowActionButtonClass('compare', true);
		expect(cls).toContain('bg-primary-500/20');
		expect(cls).toContain('border-primary-400/50');
	});
});
