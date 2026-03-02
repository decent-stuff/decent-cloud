import { describe, expect, it } from 'vitest';
import { buildOfferingDraftDiff, type OfferingDraftSnapshot } from './offering-draft-diff';

const baseSnapshot: OfferingDraftSnapshot = {
	offerName: 'Starter VPS',
	description: 'Linux server',
	productType: 'compute',
	visibility: 'private',
	isDraft: true,
	publishAt: '',
	monthlyPrice: 9.99,
	currency: 'USD',
	setupFee: 0,
	postProvisionScript: '#!/bin/bash\necho hello',
	serverType: 'cx22',
	location: 'nbg1',
	image: 'ubuntu-24.04'
};

describe('buildOfferingDraftDiff', () => {
	it('returns empty diff when effective values are unchanged', () => {
		const next = {
			...baseSnapshot,
			offerName: '  Starter VPS  ',
			description: 'Linux server  ',
			postProvisionScript: '#!/bin/bash\necho hello\n'
		};

		expect(buildOfferingDraftDiff(baseSnapshot, next)).toEqual([]);
	});

	it('returns labeled before/after rows for changed fields', () => {
		const next = {
			...baseSnapshot,
			offerName: 'Starter VPS Plus',
			monthlyPrice: 12.5,
			isDraft: false,
			publishAt: '2026-03-02T14:00'
		};

		expect(buildOfferingDraftDiff(baseSnapshot, next)).toEqual([
			{
				key: 'offerName',
				label: 'Offer Name',
				before: 'Starter VPS',
				after: 'Starter VPS Plus'
			},
			{
				key: 'isDraft',
				label: 'Listing State',
				before: 'Draft',
				after: 'Published'
			},
			{
				key: 'monthlyPrice',
				label: 'Monthly Price',
				before: '9.99 USD',
				after: '12.50 USD'
			}
		]);
	});

	it('ignores publish schedule differences when listing is not in draft mode', () => {
		const before = {
			...baseSnapshot,
			isDraft: false,
			publishAt: ''
		};
		const next = {
			...before,
			publishAt: '2026-03-02T14:00'
		};

		expect(buildOfferingDraftDiff(before, next)).toEqual([]);
	});
});
