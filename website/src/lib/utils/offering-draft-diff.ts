export interface OfferingDraftSnapshot {
	offerName: string;
	description: string;
	productType: string;
	visibility: string;
	isDraft: boolean;
	publishAt: string;
	monthlyPrice: number | null;
	currency: string;
	setupFee: number;
	postProvisionScript: string;
	serverType: string;
	location: string;
	image: string;
}

export interface OfferingDraftDiffRow {
	key: keyof OfferingDraftSnapshot;
	label: string;
	before: string;
	after: string;
}

interface DraftFieldDef {
	key: keyof OfferingDraftSnapshot;
	label: string;
	format: (snapshot: OfferingDraftSnapshot) => string;
}

const PRODUCT_TYPE_LABELS: Record<string, string> = {
	compute: 'Compute',
	gpu: 'GPU',
	storage: 'Storage',
	network: 'Network',
	dedicated: 'Dedicated'
};

const VISIBILITY_LABELS: Record<string, string> = {
	private: 'Private',
	public: 'Public',
	shared: 'Shared'
};

const DIFF_FIELDS: DraftFieldDef[] = [
	{ key: 'offerName', label: 'Offer Name', format: (snapshot) => formatText(snapshot.offerName) },
	{ key: 'description', label: 'Description', format: (snapshot) => formatText(snapshot.description) },
	{
		key: 'productType',
		label: 'Product Type',
		format: (snapshot) => PRODUCT_TYPE_LABELS[snapshot.productType] ?? formatTitleCase(snapshot.productType)
	},
	{
		key: 'visibility',
		label: 'Visibility',
		format: (snapshot) => VISIBILITY_LABELS[snapshot.visibility] ?? formatTitleCase(snapshot.visibility)
	},
	{
		key: 'isDraft',
		label: 'Listing State',
		format: (snapshot) => (snapshot.isDraft ? 'Draft' : 'Published')
	},
	{
		key: 'publishAt',
		label: 'Scheduled Publish',
		format: (snapshot) => formatPublishAt(snapshot.publishAt)
	},
	{
		key: 'monthlyPrice',
		label: 'Monthly Price',
		format: (snapshot) => formatMoney(snapshot.monthlyPrice, snapshot.currency)
	},
	{ key: 'currency', label: 'Currency', format: (snapshot) => formatText(snapshot.currency) },
	{
		key: 'setupFee',
		label: 'Setup Fee',
		format: (snapshot) => formatMoney(snapshot.setupFee, snapshot.currency)
	},
	{
		key: 'postProvisionScript',
		label: 'Post-Provision Script',
		format: (snapshot) => formatText(snapshot.postProvisionScript)
	},
	{ key: 'serverType', label: 'Server Type', format: (snapshot) => formatText(snapshot.serverType) },
	{ key: 'location', label: 'Location', format: (snapshot) => formatText(snapshot.location) },
	{ key: 'image', label: 'Image', format: (snapshot) => formatText(snapshot.image) }
];

export function buildOfferingDraftDiff(
	before: OfferingDraftSnapshot,
	after: OfferingDraftSnapshot
): OfferingDraftDiffRow[] {
	const previous = normalizeSnapshot(before);
	const next = normalizeSnapshot(after);

	return DIFF_FIELDS.filter((field) => field.format(previous) !== field.format(next)).map((field) => ({
		key: field.key,
		label: field.label,
		before: field.format(previous),
		after: field.format(next)
	}));
}

function normalizeSnapshot(snapshot: OfferingDraftSnapshot): OfferingDraftSnapshot {
	const isDraft = Boolean(snapshot.isDraft);

	return {
		...snapshot,
		offerName: (snapshot.offerName ?? '').trim(),
		description: (snapshot.description ?? '').trim(),
		productType: (snapshot.productType ?? '').trim().toLowerCase(),
		visibility: (snapshot.visibility ?? '').trim().toLowerCase(),
		isDraft,
		publishAt: isDraft ? (snapshot.publishAt ?? '').trim() : '',
		currency: (snapshot.currency ?? '').trim().toUpperCase(),
		postProvisionScript: (snapshot.postProvisionScript ?? '').trim(),
		serverType: (snapshot.serverType ?? '').trim(),
		location: (snapshot.location ?? '').trim(),
		image: (snapshot.image ?? '').trim()
	};
}

function formatText(value: string): string {
	return value === '' ? 'Not set' : value;
}

function formatPublishAt(value: string): string {
	return value === '' ? 'Not scheduled' : value.replace('T', ' ');
}

function formatMoney(amount: number | null, currency: string): string {
	if (amount === null || Number.isNaN(amount)) {
		return 'Not set';
	}
	return `${amount.toFixed(2)} ${currency}`;
}

function formatTitleCase(value: string): string {
	if (value === '') {
		return 'Not set';
	}
	return value
		.split(/[_\s-]+/)
		.filter(Boolean)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
		.join(' ');
}
