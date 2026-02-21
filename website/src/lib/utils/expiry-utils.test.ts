import { describe, it, expect } from 'vitest';
import { getExpiringContracts, isUrgent, getExpiryBannerText } from './expiry-utils';
import type { UserActivity } from '$lib/services/api-user-activity';

type Contract = UserActivity['rentals_as_requester'][number];

function makeContract(status: string, end_timestamp_ns?: number): Contract {
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

const NOW_MS = 1_000_000_000; // fixed reference point

const DAY_MS = 24 * 60 * 60 * 1000;

// ---- getExpiringContracts ----

describe('getExpiringContracts', () => {
	it('returns empty array when given no contracts', () => {
		expect(getExpiringContracts([], NOW_MS)).toEqual([]);
	});

	it('returns active/provisioned contracts expiring within 7 days', () => {
		const inSixDays = (NOW_MS + 6 * DAY_MS) * 1_000_000;
		const contracts = [
			makeContract('active', inSixDays),
			makeContract('provisioned', inSixDays),
		];
		expect(getExpiringContracts(contracts, NOW_MS)).toHaveLength(2);
	});

	it('excludes contracts expiring after 7 days', () => {
		const inEightDays = (NOW_MS + 8 * DAY_MS) * 1_000_000;
		const contracts = [makeContract('active', inEightDays)];
		expect(getExpiringContracts(contracts, NOW_MS)).toHaveLength(0);
	});

	it('excludes cancelled and expired contracts', () => {
		const inOneDayNs = (NOW_MS + DAY_MS) * 1_000_000;
		const contracts = [
			makeContract('cancelled', inOneDayNs),
			makeContract('rejected', inOneDayNs),
			makeContract('failed', inOneDayNs),
			makeContract('pending', inOneDayNs),
		];
		expect(getExpiringContracts(contracts, NOW_MS)).toHaveLength(0);
	});
});

// ---- isUrgent ----

describe('isUrgent', () => {
	it('returns true if contract expires within 24 hours', () => {
		const in12HoursNs = (NOW_MS + 12 * 60 * 60 * 1000) * 1_000_000;
		const contract = makeContract('active', in12HoursNs);
		expect(isUrgent(contract, NOW_MS)).toBe(true);
	});

	it('returns false if contract expires in 48 hours', () => {
		const in48HoursNs = (NOW_MS + 48 * 60 * 60 * 1000) * 1_000_000;
		const contract = makeContract('active', in48HoursNs);
		expect(isUrgent(contract, NOW_MS)).toBe(false);
	});

	it('returns false when end_timestamp_ns is undefined', () => {
		const contract = makeContract('active', undefined);
		expect(isUrgent(contract, NOW_MS)).toBe(false);
	});
});

// ---- getExpiryBannerText ----

describe('getExpiryBannerText', () => {
	it('formats singular message with no urgency', () => {
		expect(getExpiryBannerText(1, false)).toBe('1 contract expiring within 7 days');
	});

	it('formats plural message with urgency', () => {
		expect(getExpiryBannerText(2, true)).toBe('2 contracts expiring soon — action required within 24h');
	});

	it('formats plural message with no urgency', () => {
		expect(getExpiryBannerText(3, false)).toBe('3 contracts expiring within 7 days');
	});

	it('formats singular urgent message', () => {
		expect(getExpiryBannerText(1, true)).toBe('1 contract expiring soon — action required within 24h');
	});
});
