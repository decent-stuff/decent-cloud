import { COMPARE_MAX } from './compare';

const COMPARE_PATH = '/dashboard/marketplace/compare';

function normalizePositiveIntegerIds(values: Iterable<unknown>): number[] {
	const seen = new Set<number>();
	const normalized: number[] = [];

	for (const value of values) {
		if (typeof value !== 'number' || !Number.isSafeInteger(value) || value <= 0 || seen.has(value)) {
			continue;
		}

		seen.add(value);
		normalized.push(value);
		if (normalized.length >= COMPARE_MAX) {
			break;
		}
	}

	return normalized;
}

export function normalizeCompareIds(rawIds: string): number[] {
	const parsed = rawIds
		.split(',')
		.map((token) => token.trim())
		.filter((token) => /^\d+$/.test(token))
		.map((token) => Number(token));

	return normalizePositiveIntegerIds(parsed);
}

export function buildComparePath(ids: Iterable<number>): string {
	const canonicalIds = normalizePositiveIntegerIds(ids);
	return `${COMPARE_PATH}?ids=${canonicalIds.join(',')}`;
}

export async function copyCompareShareUrl(input: {
	ids: Iterable<number>;
	origin: string;
	clipboard: Pick<Clipboard, 'writeText'>;
}): Promise<string> {
	const path = buildComparePath(input.ids);
	const absoluteUrl = new URL(path, input.origin).toString();
	await input.clipboard.writeText(absoluteUrl);
	return absoluteUrl;
}
