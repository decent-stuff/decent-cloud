import { describe, expect, it } from 'vitest';

import { collectSavedOfferingPriceChanges } from './saved-offering-price-change';

describe('collectSavedOfferingPriceChanges', () => {
	it('collects unread saved-offering indicators and unread ids', () => {
		const result = collectSavedOfferingPriceChanges([
			{
				id: 1,
				notificationType: 'saved_offering_price_change',
				title: 'Saved offering price down',
				body: 'Offer A: monthly_price from USD 12.00 to USD 10.00.',
				offeringId: 42,
				priceDirection: 'down',
				createdAt: 1
			},
			{
				id: 2,
				notificationType: 'saved_offering_price_change',
				title: 'Saved offering price up',
				body: 'Offer B: monthly_price from USD 10.00 to USD 12.00.',
				offeringId: 99,
				priceDirection: 'up',
				createdAt: 2
			}
		]);

		expect(result.unreadNotificationIds).toEqual([1, 2]);
		expect(Array.from(result.byOfferingId.entries())).toEqual([
			[42, 'down'],
			[99, 'up']
		]);
	});

	it('ignores read, unrelated, missing-offering, and duplicate notifications', () => {
		const result = collectSavedOfferingPriceChanges([
			{
				id: 1,
				notificationType: 'saved_offering_price_change',
				title: 'Saved offering price down',
				body: 'Offer A: monthly_price from USD 12.00 to USD 10.00.',
				offeringId: 42,
				priceDirection: 'down',
				createdAt: 1,
				readAt: 5
			},
			{
				id: 2,
				notificationType: 'contract_status',
				title: 'Contract active',
				body: 'Body',
				createdAt: 2
			},
			{
				id: 3,
				notificationType: 'saved_offering_price_change',
				title: 'Saved offering price down',
				body: 'Offer B: monthly_price from USD 12.00 to USD 10.00.',
				priceDirection: 'down',
				createdAt: 3
			},
			{
				id: 4,
				notificationType: 'saved_offering_price_change',
				title: 'Saved offering price down',
				body: 'Offer C: monthly_price from USD 12.00 to USD 10.00.',
				offeringId: 7,
				priceDirection: 'down',
				createdAt: 4
			},
			{
				id: 5,
				notificationType: 'saved_offering_price_change',
				title: 'Saved offering price up',
				body: 'Offer C: monthly_price from USD 10.00 to USD 12.00.',
				offeringId: 7,
				priceDirection: 'up',
				createdAt: 5
			}
		]);

		expect(result.unreadNotificationIds).toEqual([4, 5]);
		expect(Array.from(result.byOfferingId.entries())).toEqual([[7, 'down']]);
	});

	it('prefers structured direction over title text', () => {
		const result = collectSavedOfferingPriceChanges([
			{
				id: 1,
				notificationType: 'saved_offering_price_change',
				title: 'Saved offering price up',
				body: 'Offer D: monthly_price from USD 12.00 to USD 10.00.',
				offeringId: 55,
				priceDirection: 'down',
				createdAt: 1
			}
		]);

		expect(Array.from(result.byOfferingId.entries())).toEqual([[55, 'down']]);
	});
});
