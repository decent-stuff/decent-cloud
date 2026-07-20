import { test, expect } from '@playwright/test';

/**
 * E2E coverage for /agents.
 *
 * Public marketing page for the "Decent Agents" hosted-AI-agent product.
 * Anonymous-OK — no auth required. The page is fully static (only the
 * waitlist form posts to a public API endpoint), so all assertions are
 * against hard-coded marketing copy that only renders when this route
 * actually mounts.
 */

test.describe('/agents', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/agents');
	});

	test('renders hero with EU-hosted badge and waitlist form', async ({ page }) => {
		// EU trust badge — exact match because the pricing card also says
		// "EU-hosted on Hetzner · B2B-first" (substring match would be ambiguous).
		await expect(page.getByText('EU-hosted on Hetzner', { exact: true })).toBeVisible();

		// Hero headline (split across two spans, single h1)
		await expect(page.getByRole('heading', { name: /Rent an AI engineer for your GitHub repo/i })).toBeVisible();
		await expect(page.getByText(/Hosted AI agents that work your GitHub backlog 24\/7/i)).toBeVisible();

		// Waitlist form is unique to this page — assert its labelled inputs
		await expect(page.getByLabel('Work email')).toBeVisible();
		await expect(page.getByLabel('GitHub handle')).toBeVisible();
		await expect(page.getByRole('button', { name: /Start beta/i }).first()).toBeVisible();
	});

	test('renders the three-step "how it works" section', async ({ page }) => {
		await expect(page.getByRole('heading', { name: 'Three steps. No glue code.' })).toBeVisible();

		// The three step titles come from the `steps` array in the svelte source
		await expect(page.getByRole('heading', { name: 'Connect your repo' })).toBeVisible();
		await expect(page.getByRole('heading', { name: 'File an issue' })).toBeVisible();
		await expect(page.getByRole('heading', { name: 'Get a PR' })).toBeVisible();
	});

	test('renders pricing section with link to detailed pricing page', async ({ page }) => {
		await expect(page.getByRole('heading', { name: 'One tier. No surprises.' })).toBeVisible();

		// Link to /agents/pricing — unique to this route
		await expect(page.getByRole('link', { name: /See pricing details and usage assumptions/i })).toHaveAttribute(
			'href',
			'/agents/pricing',
		);
	});

	test('renders FAQ section with expected questions', async ({ page }) => {
		await expect(page.getByRole('heading', { name: 'Common questions' })).toBeVisible();

		// FAQ questions live inside <summary> (not headings). They're rendered
		// as the summary text of each <details> element.
		const faq1 = page.locator('summary', { hasText: 'How does it actually work?' });
		const faq2 = page.locator('summary', { hasText: 'Can I cancel any time?' });
		const faq3 = page.locator('summary', { hasText: 'Where is it hosted?' });
		await expect(faq1).toHaveCount(1);
		await expect(faq2).toHaveCount(1);
		await expect(faq3).toHaveCount(1);
	});
});
