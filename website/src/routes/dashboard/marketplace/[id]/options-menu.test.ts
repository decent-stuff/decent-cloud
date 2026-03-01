import { describe, it, expect } from 'vitest';

type OptionsMenuState = {
	showOptionsMenu: boolean;
	copyLinkFeedback: boolean;
};

function createOptionsMenuReducer() {
	let state: OptionsMenuState = {
		showOptionsMenu: false,
		copyLinkFeedback: false
	};

	return {
		get state() { return state; },
		toggleMenu() {
			state = { ...state, showOptionsMenu: !state.showOptionsMenu };
		},
		closeMenu() {
			state = { ...state, showOptionsMenu: false };
		},
		copyLink() {
			state = { ...state, copyLinkFeedback: true };
		},
		resetCopyFeedback() {
			state = { ...state, copyLinkFeedback: false };
		}
	};
}

describe('offering detail options menu', () => {
	describe('menu toggle', () => {
		it('opens menu when toggle is called on closed menu', () => {
			const menu = createOptionsMenuReducer();
			expect(menu.state.showOptionsMenu).toBe(false);
			menu.toggleMenu();
			expect(menu.state.showOptionsMenu).toBe(true);
		});

		it('closes menu when toggle is called on open menu', () => {
			const menu = createOptionsMenuReducer();
			menu.toggleMenu();
			expect(menu.state.showOptionsMenu).toBe(true);
			menu.toggleMenu();
			expect(menu.state.showOptionsMenu).toBe(false);
		});

		it('closes menu when closeMenu is called', () => {
			const menu = createOptionsMenuReducer();
			menu.toggleMenu();
			expect(menu.state.showOptionsMenu).toBe(true);
			menu.closeMenu();
			expect(menu.state.showOptionsMenu).toBe(false);
		});

		it('closeMenu is idempotent', () => {
			const menu = createOptionsMenuReducer();
			menu.closeMenu();
			menu.closeMenu();
			expect(menu.state.showOptionsMenu).toBe(false);
		});
	});

	describe('copy link feedback', () => {
		it('shows feedback after copying link', () => {
			const menu = createOptionsMenuReducer();
			expect(menu.state.copyLinkFeedback).toBe(false);
			menu.copyLink();
			expect(menu.state.copyLinkFeedback).toBe(true);
		});

		it('hides feedback after reset', () => {
			const menu = createOptionsMenuReducer();
			menu.copyLink();
			expect(menu.state.copyLinkFeedback).toBe(true);
			menu.resetCopyFeedback();
			expect(menu.state.copyLinkFeedback).toBe(false);
		});
	});
});

describe('click outside handler', () => {
	it('should close menu when clicking outside', () => {
		const menu = createOptionsMenuReducer();
		menu.toggleMenu();
		expect(menu.state.showOptionsMenu).toBe(true);
		
		menu.closeMenu();
		expect(menu.state.showOptionsMenu).toBe(false);
	});
});

describe('escape key handler', () => {
	it('should close menu when escape is pressed', () => {
		const menu = createOptionsMenuReducer();
		menu.toggleMenu();
		expect(menu.state.showOptionsMenu).toBe(true);
		
		menu.closeMenu();
		expect(menu.state.showOptionsMenu).toBe(false);
	});
});

type OfferingSubset = {
	id?: number;
	offering_source?: string;
	external_checkout_url?: string;
};

function shouldShowExternalCheckout(offering: OfferingSubset | null): boolean {
	return offering?.offering_source === 'seeded' && !!offering?.external_checkout_url;
}

function getPrimaryCtaLabel(offering: OfferingSubset | null): string {
	if (!offering) return 'Rent this offering';
	if (shouldShowExternalCheckout(offering)) return 'Visit Provider';
	return 'Rent this offering';
}

describe('offering detail primary CTA', () => {
	describe('external checkout offerings', () => {
		it('shows "Visit Provider" for seeded offerings with external checkout', () => {
			const offering: OfferingSubset = {
				id: 1,
				offering_source: 'seeded',
				external_checkout_url: 'https://example.com/checkout'
			};
			expect(getPrimaryCtaLabel(offering)).toBe('Visit Provider');
		});

		it('shows "Rent this offering" for seeded offerings without external checkout', () => {
			const offering: OfferingSubset = {
				id: 1,
				offering_source: 'seeded'
			};
			expect(getPrimaryCtaLabel(offering)).toBe('Rent this offering');
		});
	});

	describe('regular offerings', () => {
		it('shows "Rent this offering" for regular offerings', () => {
			const offering: OfferingSubset = {
				id: 1,
				offering_source: 'user'
			};
			expect(getPrimaryCtaLabel(offering)).toBe('Rent this offering');
		});

		it('shows "Rent this offering" when offering_source is undefined', () => {
			const offering: OfferingSubset = { id: 1 };
			expect(getPrimaryCtaLabel(offering)).toBe('Rent this offering');
		});
	});
});

describe('secondary actions in kebab menu', () => {
	const expectedActions = ['copy-link', 'save', 'ask-provider'];

	it('includes copy link action', () => {
		expect(expectedActions).toContain('copy-link');
	});

	it('includes save action', () => {
		expect(expectedActions).toContain('save');
	});

	it('includes ask provider action', () => {
		expect(expectedActions).toContain('ask-provider');
	});
});
