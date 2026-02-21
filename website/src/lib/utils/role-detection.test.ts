import { describe, it, expect } from 'vitest';
import {
	detectUserRole,
	countActiveRentals,
	countExpiringSoon,
	countActiveRentalsAsProvider,
} from './role-detection';
import type { UserActivity } from '$lib/services/api-user-activity';
import type { Offering } from '$lib/services/api';

// Minimal contract factory for tests
function makeContract(status: string, end_timestamp_ns?: number): UserActivity['rentals_as_requester'][number] {
	return {
		contract_id: 'test-id',
		requester_pubkey: 'abc',
		requester_ssh_pubkey: '',
		requester_contact: '',
		provider_pubkey: 'def',
		offering_id: 'offer-1',
		region_name: null,
		instance_config: null,
		payment_amount_e9s: 1_000_000_000,
		start_timestamp_ns: undefined,
		end_timestamp_ns,
		duration_hours: undefined,
		original_duration_hours: undefined,
		request_memo: '',
		created_at_ns: 0,
		status,
		provisioning_instance_details: null,
		provisioning_completed_at_ns: undefined,
		payment_method: 'icp',
		stripe_payment_intent_id: null,
		stripe_customer_id: null,
		icpay_transaction_id: null,
		payment_status: 'paid',
		currency: 'ICP',
		refund_amount_e9s: undefined,
		stripe_refund_id: null,
		refund_created_at_ns: undefined,
		status_updated_at_ns: undefined,
		icpay_payment_id: null,
		icpay_refund_id: null,
		total_released_e9s: undefined,
		last_release_at_ns: undefined,
		tax_amount_e9s: undefined,
		tax_rate_percent: undefined,
		tax_type: null,
		tax_jurisdiction: null,
		customer_tax_id: null,
		reverse_charge: null,
		buyer_address: null,
		stripe_invoice_id: null,
		receipt_number: undefined,
		receipt_sent_at_ns: undefined,
		stripe_subscription_id: null,
		subscription_status: null,
		current_period_end_ns: undefined,
		cancel_at_period_end: false,
		auto_renew: false,
		gateway_slug: null,
		gateway_subdomain: null,
		gateway_ssh_port: undefined,
		gateway_port_range_start: undefined,
		gateway_port_range_end: undefined,
		password_reset_requested_at_ns: undefined,
	};
}

function makeActivity(overrides: Partial<UserActivity> = {}): UserActivity {
	return {
		offerings_provided: [],
		rentals_as_requester: [],
		rentals_as_provider: [],
		...overrides,
	};
}

function makeOffering(id: number): Offering {
	return {
		id,
		pubkey: 'abc',
		offer_name: `Offering ${id}`,
		monthly_price: 10,
		currency: 'ICP',
		datacenter_country: 'US',
		visibility: 'public',
		processor_cores: undefined,
		memory_amount: undefined,
		total_ssd_capacity: undefined,
		resolved_pool_id: undefined,
		resolved_pool_name: undefined,
	} as unknown as Offering;
}

// ---- detectUserRole ----

describe('detectUserRole: new user', () => {
	it('returns "new" when activity is null and no offerings', () => {
		expect(detectUserRole(null, [])).toBe('new');
	});

	it('returns "new" when activity has empty arrays and no offerings', () => {
		expect(detectUserRole(makeActivity(), [])).toBe('new');
	});

	it('returns "new" when activity has only provider rentals but no own offerings', () => {
		// rentals_as_provider alone does NOT make a "tenant" - requester contracts do
		const activity = makeActivity({
			rentals_as_provider: [makeContract('active')],
		});
		expect(detectUserRole(activity, [])).toBe('new');
	});
});

describe('detectUserRole: provider', () => {
	it('returns "provider" when user has 1 offering', () => {
		expect(detectUserRole(null, [makeOffering(1)])).toBe('provider');
	});

	it('returns "provider" when user has multiple offerings', () => {
		expect(detectUserRole(makeActivity(), [makeOffering(1), makeOffering(2)])).toBe('provider');
	});

	it('returns "provider" even when user also has tenant contracts', () => {
		const activity = makeActivity({ rentals_as_requester: [makeContract('active')] });
		expect(detectUserRole(activity, [makeOffering(1)])).toBe('provider');
	});
});

describe('detectUserRole: tenant', () => {
	it('returns "tenant" when user has contracts but no offerings', () => {
		const activity = makeActivity({ rentals_as_requester: [makeContract('active')] });
		expect(detectUserRole(activity, [])).toBe('tenant');
	});

	it('returns "tenant" when offerings_provided has entries but no myOfferings', () => {
		const activity = makeActivity({ offerings_provided: [makeOffering(1)] as any });
		expect(detectUserRole(activity, [])).toBe('tenant');
	});
});

// ---- countActiveRentals ----

describe('countActiveRentals', () => {
	it('returns 0 when activity is null', () => {
		expect(countActiveRentals(null)).toBe(0);
	});

	it('returns 0 when no rentals', () => {
		expect(countActiveRentals(makeActivity())).toBe(0);
	});

	it('counts active and provisioned contracts', () => {
		const activity = makeActivity({
			rentals_as_requester: [
				makeContract('active'),
				makeContract('provisioned'),
				makeContract('cancelled'),
				makeContract('pending'),
			],
		});
		expect(countActiveRentals(activity)).toBe(2);
	});

	it('does not count terminal statuses', () => {
		const activity = makeActivity({
			rentals_as_requester: [makeContract('cancelled'), makeContract('rejected'), makeContract('failed')],
		});
		expect(countActiveRentals(activity)).toBe(0);
	});
});

// ---- countExpiringSoon ----

describe('countExpiringSoon', () => {
	const nowMs = Date.now();

	it('returns 0 when activity is null', () => {
		expect(countExpiringSoon(null, 7)).toBe(0);
	});

	it('returns 0 when no active contracts', () => {
		const activity = makeActivity({ rentals_as_requester: [makeContract('cancelled')] });
		expect(countExpiringSoon(activity, 7)).toBe(0);
	});

	it('counts active contracts expiring within cutoff', () => {
		const inSixDaysNs = (nowMs + 6 * 24 * 60 * 60 * 1000) * 1_000_000;
		const activity = makeActivity({
			rentals_as_requester: [makeContract('active', inSixDaysNs)],
		});
		expect(countExpiringSoon(activity, 7)).toBe(1);
	});

	it('does not count contracts expiring after cutoff', () => {
		const inEightDaysNs = (nowMs + 8 * 24 * 60 * 60 * 1000) * 1_000_000;
		const activity = makeActivity({
			rentals_as_requester: [makeContract('active', inEightDaysNs)],
		});
		expect(countExpiringSoon(activity, 7)).toBe(0);
	});

	it('does not count already-expired contracts', () => {
		const alreadyExpiredNs = (nowMs - 1000) * 1_000_000;
		const activity = makeActivity({
			rentals_as_requester: [makeContract('active', alreadyExpiredNs)],
		});
		expect(countExpiringSoon(activity, 7)).toBe(0);
	});

	it('does not count contracts without end_timestamp_ns', () => {
		const activity = makeActivity({
			rentals_as_requester: [makeContract('active', undefined)],
		});
		expect(countExpiringSoon(activity, 7)).toBe(0);
	});

	it('does not count non-active contracts even if within cutoff', () => {
		const inOneDayNs = (nowMs + 1 * 24 * 60 * 60 * 1000) * 1_000_000;
		const activity = makeActivity({
			rentals_as_requester: [makeContract('pending', inOneDayNs)],
		});
		expect(countExpiringSoon(activity, 7)).toBe(0);
	});
});

// ---- countActiveRentalsAsProvider ----

describe('countActiveRentalsAsProvider', () => {
	it('returns 0 when activity is null', () => {
		expect(countActiveRentalsAsProvider(null)).toBe(0);
	});

	it('returns 0 when no provider rentals', () => {
		expect(countActiveRentalsAsProvider(makeActivity())).toBe(0);
	});

	it('counts active and provisioned contracts as provider', () => {
		const activity = makeActivity({
			rentals_as_provider: [
				makeContract('active'),
				makeContract('provisioned'),
				makeContract('cancelled'),
			],
		});
		expect(countActiveRentalsAsProvider(activity)).toBe(2);
	});

	it('does not include requester contracts in provider count', () => {
		const activity = makeActivity({
			rentals_as_requester: [makeContract('active'), makeContract('active')],
			rentals_as_provider: [makeContract('provisioned')],
		});
		expect(countActiveRentalsAsProvider(activity)).toBe(1);
	});
});
