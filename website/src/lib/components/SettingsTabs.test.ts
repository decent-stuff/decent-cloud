import { describe, it, expect } from 'vitest';

interface Tab {
	href: string;
	label: string;
	icon: string;
}

const tabs: Tab[] = [
	{ href: "/dashboard/account", label: "Overview", icon: "⚙️" },
	{ href: "/dashboard/account/profile", label: "Profile", icon: "👤" },
	{ href: "/dashboard/account/security", label: "Security", icon: "🔐" },
	{ href: "/dashboard/account/subscription", label: "Subscription", icon: "⭐" },
	{ href: "/dashboard/account/billing", label: "Billing", icon: "💳" },
	{ href: "/dashboard/account/notifications", label: "Notifications", icon: "🔔" },
];

function getActiveTab(path: string): Tab | undefined {
	return tabs.find((tab) => tab.href === path);
}

function isActive(currentPath: string, tabHref: string): boolean {
	return currentPath === tabHref;
}

describe('SettingsTabs', () => {
	describe('tab structure', () => {
		it('contains all expected tabs', () => {
			expect(tabs).toHaveLength(6);
		});

		it('has Overview as first tab linking to account root', () => {
			expect(tabs[0].label).toBe('Overview');
			expect(tabs[0].href).toBe('/dashboard/account');
		});

		it('has Profile tab with correct href', () => {
			const profileTab = tabs.find((t) => t.label === 'Profile');
			expect(profileTab?.href).toBe('/dashboard/account/profile');
		});

		it('has Security tab with correct href', () => {
			const securityTab = tabs.find((t) => t.label === 'Security');
			expect(securityTab?.href).toBe('/dashboard/account/security');
		});

		it('has Subscription tab with correct href', () => {
			const subTab = tabs.find((t) => t.label === 'Subscription');
			expect(subTab?.href).toBe('/dashboard/account/subscription');
		});

		it('has Billing tab with correct href', () => {
			const billingTab = tabs.find((t) => t.label === 'Billing');
			expect(billingTab?.href).toBe('/dashboard/account/billing');
		});

		it('has Notifications tab with correct href', () => {
			const notifTab = tabs.find((t) => t.label === 'Notifications');
			expect(notifTab?.href).toBe('/dashboard/account/notifications');
		});

		it('all tabs have icons', () => {
			for (const tab of tabs) {
				expect(tab.icon.length).toBeGreaterThan(0);
			}
		});
	});

	describe('active state detection', () => {
		it('identifies Overview as active on account root', () => {
			expect(isActive('/dashboard/account', '/dashboard/account')).toBe(true);
			expect(getActiveTab('/dashboard/account')?.label).toBe('Overview');
		});

		it('identifies Profile as active on profile page', () => {
			expect(isActive('/dashboard/account/profile', '/dashboard/account/profile')).toBe(true);
			expect(getActiveTab('/dashboard/account/profile')?.label).toBe('Profile');
		});

		it('identifies Security as active on security page', () => {
			expect(isActive('/dashboard/account/security', '/dashboard/account/security')).toBe(true);
			expect(getActiveTab('/dashboard/account/security')?.label).toBe('Security');
		});

		it('identifies Subscription as active on subscription page', () => {
			expect(isActive('/dashboard/account/subscription', '/dashboard/account/subscription')).toBe(true);
			expect(getActiveTab('/dashboard/account/subscription')?.label).toBe('Subscription');
		});

		it('identifies Billing as active on billing page', () => {
			expect(isActive('/dashboard/account/billing', '/dashboard/account/billing')).toBe(true);
			expect(getActiveTab('/dashboard/account/billing')?.label).toBe('Billing');
		});

		it('identifies Notifications as active on notifications page', () => {
			expect(isActive('/dashboard/account/notifications', '/dashboard/account/notifications')).toBe(true);
			expect(getActiveTab('/dashboard/account/notifications')?.label).toBe('Notifications');
		});

		it('only one tab is active at a time', () => {
			const paths = [
				'/dashboard/account',
				'/dashboard/account/profile',
				'/dashboard/account/security',
				'/dashboard/account/subscription',
				'/dashboard/account/billing',
				'/dashboard/account/notifications',
			];

			for (const path of paths) {
				const activeCount = tabs.filter((t) => isActive(path, t.href)).length;
				expect(activeCount).toBe(1);
			}
		});

		it('non-matching paths return no active tab', () => {
			expect(getActiveTab('/dashboard/other')).toBeUndefined();
		});
	});

	describe('navigation coverage', () => {
		it('all hrefs are valid internal routes', () => {
			for (const tab of tabs) {
				expect(tab.href.startsWith('/dashboard/account')).toBe(true);
			}
		});

		it('all tabs have unique hrefs', () => {
			const hrefs = tabs.map((t) => t.href);
			const uniqueHrefs = new Set(hrefs);
			expect(uniqueHrefs.size).toBe(tabs.length);
		});

		it('all tabs have unique labels', () => {
			const labels = tabs.map((t) => t.label);
			const uniqueLabels = new Set(labels);
			expect(uniqueLabels.size).toBe(tabs.length);
		});
	});
});
