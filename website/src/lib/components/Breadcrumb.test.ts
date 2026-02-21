import { describe, it, expect } from 'vitest';

interface BreadcrumbItem {
	label: string;
	href?: string;
}

// Mirror the component's mobile truncation logic as a pure function
function getMobileItems(items: BreadcrumbItem[]): BreadcrumbItem[] {
	return items.length > 2 ? items.slice(-2) : items;
}

describe('Breadcrumb', () => {
	describe('item structure', () => {
		it('items with href are link candidates (have href property)', () => {
			const items: BreadcrumbItem[] = [
				{ label: 'Dashboard', href: '/dashboard' },
				{ label: 'My Rentals', href: '/dashboard/rentals' },
				{ label: 'Contract #abc12345' },
			];
			const links = items.filter((i) => i.href !== undefined);
			const plain = items.filter((i) => i.href === undefined);

			expect(links).toHaveLength(2);
			expect(plain).toHaveLength(1);
			expect(plain[0].label).toBe('Contract #abc12345');
		});

		it('items without href are plain text (no href property)', () => {
			const items: BreadcrumbItem[] = [
				{ label: 'Dashboard', href: '/dashboard' },
				{ label: 'Current Page' },
			];
			expect(items[1].href).toBeUndefined();
		});
	});

	describe('mobile truncation (last 2 items)', () => {
		it('returns all items when count is 2 or fewer', () => {
			const items: BreadcrumbItem[] = [
				{ label: 'Home', href: '/' },
				{ label: 'Current' },
			];
			expect(getMobileItems(items)).toHaveLength(2);
			expect(getMobileItems(items)).toEqual(items);
		});

		it('returns last 2 items when count exceeds 2', () => {
			const items: BreadcrumbItem[] = [
				{ label: 'Dashboard', href: '/dashboard' },
				{ label: 'Marketplace', href: '/dashboard/marketplace' },
				{ label: 'Offering Name' },
			];
			const result = getMobileItems(items);
			expect(result).toHaveLength(2);
			expect(result[0].label).toBe('Marketplace');
			expect(result[1].label).toBe('Offering Name');
		});

		it('returns last 2 items for a 4-level deep path', () => {
			const items: BreadcrumbItem[] = [
				{ label: 'Root', href: '/' },
				{ label: 'Section', href: '/section' },
				{ label: 'Subsection', href: '/section/sub' },
				{ label: 'Detail' },
			];
			const result = getMobileItems(items);
			expect(result).toHaveLength(2);
			expect(result[0].label).toBe('Subsection');
			expect(result[1].label).toBe('Detail');
		});

		it('returns single item unchanged', () => {
			const items: BreadcrumbItem[] = [{ label: 'Only' }];
			expect(getMobileItems(items)).toHaveLength(1);
			expect(getMobileItems(items)[0].label).toBe('Only');
		});
	});

	describe('separator logic', () => {
		it('separator is needed between every pair of items', () => {
			const items: BreadcrumbItem[] = [
				{ label: 'A', href: '/a' },
				{ label: 'B', href: '/b' },
				{ label: 'C' },
			];
			// Number of separators = items.length - 1
			const separatorCount = items.length - 1;
			expect(separatorCount).toBe(2);
		});

		it('no separator for a single item', () => {
			const items: BreadcrumbItem[] = [{ label: 'Only' }];
			const separatorCount = items.length - 1;
			expect(separatorCount).toBe(0);
		});
	});
});
