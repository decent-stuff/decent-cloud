export type DashboardCtaKind =
	| 'marketplace-active-filter-chip'
	| 'marketplace-active-filter-clear'
	| 'marketplace-sort-pill'
	| 'rentals-empty-state-cta'
	| 'rentals-contract-action-primary'
	| 'rentals-contract-action-secondary'
	| 'rentals-contract-action-warning'
	| 'rentals-pending-guidance-view'
	| 'rentals-pending-guidance-dismiss';

type DashboardCtaPeer = {
	kind: DashboardCtaKind;
	iconOnly?: boolean;
};

const CTA_BASE = 'inline-flex items-center justify-center whitespace-nowrap leading-none transition-colors';

const CLASSES_BY_KIND: Record<DashboardCtaKind, string> = {
	'marketplace-active-filter-chip': `${CTA_BASE} h-7 min-h-7 gap-1 px-2 text-xs bg-primary-500/20 text-primary-400 border border-primary-500/30 rounded hover:bg-primary-500/30`,
	'marketplace-active-filter-clear': `${CTA_BASE} h-7 min-h-7 px-2 text-xs text-neutral-500 hover:text-white`,
	'marketplace-sort-pill': `${CTA_BASE} h-7 min-h-7 px-2 text-xs rounded`,
	'rentals-empty-state-cta': `${CTA_BASE} btn-control-md w-full sm:w-auto gap-2 font-semibold`,
	'rentals-contract-action-primary': `${CTA_BASE} h-7 min-h-7 gap-1 px-2 text-xs text-white rounded`,
	'rentals-contract-action-secondary': `${CTA_BASE} h-7 min-h-7 gap-1 px-2 text-xs text-white rounded`,
	'rentals-contract-action-warning': `${CTA_BASE} h-7 min-h-7 px-2 text-xs rounded border`,
	'rentals-pending-guidance-view': `${CTA_BASE} h-7 min-h-7 px-3 text-xs font-medium bg-primary-500/20 text-primary-300 border border-primary-500/30 hover:bg-primary-500/30`,
	'rentals-pending-guidance-dismiss': `${CTA_BASE} h-7 min-h-7 w-7 text-neutral-500 hover:text-neutral-300 text-lg`
};

const HEIGHT_BY_KIND: Record<DashboardCtaKind, number> = {
	'marketplace-active-filter-chip': 28,
	'marketplace-active-filter-clear': 28,
	'marketplace-sort-pill': 28,
	'rentals-empty-state-cta': 44,
	'rentals-contract-action-primary': 28,
	'rentals-contract-action-secondary': 28,
	'rentals-contract-action-warning': 28,
	'rentals-pending-guidance-view': 28,
	'rentals-pending-guidance-dismiss': 28
};

export function buildDashboardCtaClass(kind: DashboardCtaKind): string {
	return CLASSES_BY_KIND[kind];
}

export function getDashboardCtaHeightPx(kind: DashboardCtaKind): number {
	return HEIGHT_BY_KIND[kind];
}

export function assertDashboardPeerCtaHeights(
	peers: DashboardCtaPeer[],
	thresholdPx = 2
): {
	pass: boolean;
	deltaPx: number;
	measuredKinds: DashboardCtaKind[];
} {
	const measuredKinds = peers.filter((peer) => !peer.iconOnly).map((peer) => peer.kind);
	if (measuredKinds.length < 2) {
		return { pass: true, deltaPx: 0, measuredKinds };
	}

	const heights = measuredKinds.map((kind) => getDashboardCtaHeightPx(kind));
	const deltaPx = Math.max(...heights) - Math.min(...heights);

	return {
		pass: deltaPx <= thresholdPx,
		deltaPx,
		measuredKinds
	};
}
