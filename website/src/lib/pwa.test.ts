import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';

const STATIC_DIR = resolve(__dirname, '../../static');
const SRC_DIR = resolve(__dirname, '..');

describe('PWA manifest.json', () => {
	const manifestPath = resolve(STATIC_DIR, 'manifest.json');
	let manifest: {
		name: string;
		short_name: string;
		description: string;
		start_url: string;
		display: string;
		theme_color: string;
		background_color: string;
		icons: Array<{ src: string; sizes: string; type: string }>;
	};

	it('file exists and is valid JSON', () => {
		const raw = readFileSync(manifestPath, 'utf-8');
		manifest = JSON.parse(raw);
		expect(manifest).toBeDefined();
	});

	it('has required identity fields', () => {
		const raw = readFileSync(manifestPath, 'utf-8');
		manifest = JSON.parse(raw);
		expect(manifest.name).toBe('Decent Cloud');
		expect(manifest.short_name).toBe('DC');
		expect(manifest.description).toBe('Decentralized cloud infrastructure marketplace');
	});

	it('has required display and navigation fields', () => {
		const raw = readFileSync(manifestPath, 'utf-8');
		manifest = JSON.parse(raw);
		expect(manifest.start_url).toBe('/');
		expect(manifest.display).toBe('standalone');
		expect(manifest.theme_color).toBeDefined();
		expect(manifest.background_color).toBeDefined();
	});

	it('has icons with 192x192 and 512x512 sizes', () => {
		const raw = readFileSync(manifestPath, 'utf-8');
		manifest = JSON.parse(raw);
		const sizes = manifest.icons.map((i) => i.sizes);
		expect(sizes).toContain('192x192');
		expect(sizes).toContain('512x512');
		manifest.icons.forEach((icon) => {
			expect(icon.src).toBeDefined();
			expect(icon.type).toBeDefined();
		});
	});
});

describe('PWA app.html', () => {
	const appHtmlPath = resolve(SRC_DIR, 'app.html');
	let html: string;

	it('links the manifest', () => {
		html = readFileSync(appHtmlPath, 'utf-8');
		expect(html).toContain('<link rel="manifest" href="/manifest.json">');
	});

	it('has theme-color meta tag', () => {
		html = readFileSync(appHtmlPath, 'utf-8');
		expect(html).toMatch(/<meta name="theme-color"/);
	});

	it('has apple-mobile-web-app-capable meta tag', () => {
		html = readFileSync(appHtmlPath, 'utf-8');
		expect(html).toContain('<meta name="apple-mobile-web-app-capable" content="yes">');
	});

	it('registers the service worker', () => {
		html = readFileSync(appHtmlPath, 'utf-8');
		expect(html).toContain("navigator.serviceWorker.register('/sw.js')");
	});
});

describe('PWA service worker', () => {
	const swPath = resolve(STATIC_DIR, 'sw.js');

	it('file exists', () => {
		const sw = readFileSync(swPath, 'utf-8');
		expect(sw.length).toBeGreaterThan(0);
	});

	it('handles install event to cache app shell', () => {
		const sw = readFileSync(swPath, 'utf-8');
		expect(sw).toContain("addEventListener('install'");
		expect(sw).toContain('cache.addAll');
	});

	it('handles fetch event with cache-first strategy', () => {
		const sw = readFileSync(swPath, 'utf-8');
		expect(sw).toContain("addEventListener('fetch'");
		expect(sw).toContain('caches.match');
	});

	it('serves offline fallback for navigation requests', () => {
		const sw = readFileSync(swPath, 'utf-8');
		expect(sw).toContain('/offline');
	});
});
