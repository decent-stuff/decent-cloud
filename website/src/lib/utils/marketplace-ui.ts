export type QuickPillKind = 'filter' | 'preset';
export type QuickPillColor =
	| 'neutral'
	| 'amber'
	| 'purple'
	| 'emerald'
	| 'sky';

const QUICK_PILL_BASE =
	'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-full border transition-colors';

const QUICK_PILL_INACTIVE =
	'bg-neutral-800/60 text-neutral-400 border-neutral-700 hover:border-neutral-500 hover:text-white';

const ACTIVE_BY_COLOR: Record<string, string> = {
	neutral: 'bg-primary-500/20 text-primary-300 border-primary-500/50',
	amber: 'bg-warning/20 text-warning border-warning/50',
	purple: 'bg-info/20 text-info border-info/50',
	emerald: 'bg-success/20 text-success border-success/50',
	sky: 'bg-info/20 text-info border-info/50',
};

function activeColorClass(color: QuickPillColor | string): string {
	return ACTIVE_BY_COLOR[color] ?? ACTIVE_BY_COLOR.neutral;
}

export function buildQuickPillClass(
	kind: QuickPillKind,
	active: boolean,
	color: QuickPillColor | string
): string {
	const semanticClass = kind === 'filter' ? 'quick-pill-filter' : 'quick-pill-preset';
	const stateClass = active ? activeColorClass(color) : QUICK_PILL_INACTIVE;
	return `${QUICK_PILL_BASE} ${semanticClass} ${stateClass}`;
}

const ROW_ACTION_BASE = 'h-7 min-h-7 inline-flex items-center justify-center leading-none whitespace-nowrap';
const ROW_ACTION_SELECTED = 'bg-primary-500/20 text-primary-300 border-primary-400/50 hover:bg-primary-500/10';
const ROW_ACTION_DEFAULT = 'bg-neutral-800 text-neutral-400 border-neutral-700 hover:bg-neutral-700 hover:text-white';

export function buildRowActionButtonClass(
	kind: 'rent' | 'save' | 'compare',
	selected = false
): string {
	if (kind === 'rent') {
		return `${ROW_ACTION_BASE} bg-primary-600 hover:bg-primary-500`;
	}

	const stateClass = selected ? ROW_ACTION_SELECTED : ROW_ACTION_DEFAULT;
	return `${ROW_ACTION_BASE} border ${stateClass}`;
}
