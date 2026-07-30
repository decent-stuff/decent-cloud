// PWA service worker. Bump CACHE_VERSION on major app changes; activate()
// evicts every cache whose name differs, so deploys auto-heal stale clients.
const CACHE_VERSION = 'v2';
const CACHE = `decent-cloud-${CACHE_VERSION}`;
const OFFLINE_FALLBACK = '/offline';

self.addEventListener('install', (event) => {
	// Precache ONLY the static offline page — never the app shell (precaching '/'
	// was the stale-cache vector). skipWaiting() activates immediately for users.
	event.waitUntil(caches.open(CACHE).then((c) => c.add(OFFLINE_FALLBACK).catch(() => {})));
	self.skipWaiting();
});

self.addEventListener('activate', (event) => {
	event.waitUntil(
		(async () => {
			await Promise.all(
				(await caches.keys())
					.filter((key) => key !== CACHE)
					.map((key) => caches.delete(key))
			);
			await self.clients.claim();
		})()
	);
});

self.addEventListener('fetch', (event) => {
	const req = event.request;

	// non-GET: straight to network, never intercepted.
	if (req.method !== 'GET') return;

	const url = new URL(req.url);
	const sameOrigin = url.origin === self.location.origin;

	// /sw.js itself: never cache so SW updates always land.
	// cross-origin or /api/: network-only, never cached.
	if (sameOrigin && url.pathname === '/sw.js') return;
	if (!sameOrigin || url.pathname.startsWith('/api/')) return;

	// Navigations: network-first → fresh HTML every visit, offline fallback last.
	if (req.mode === 'navigate') {
		event.respondWith(
			(async () => {
				try {
					const res = await fetch(req);
					if (res.ok) {
						const cache = await caches.open(CACHE);
						cache.put(req, res.clone());
					}
					return res;
				} catch {
				const cached = await caches.match(req, { cacheName: CACHE });
				return cached || (await caches.match(OFFLINE_FALLBACK, { cacheName: CACHE }));
				}
			})()
		);
		return;
	}

	// Content-hashed SvelteKit assets: cache-first (URL changes when content does).
	if (url.pathname.startsWith('/_app/immutable/')) {
		event.respondWith(
			(async () => {
			const cached = await caches.match(req, { cacheName: CACHE });
			if (cached) return cached;
				const res = await fetch(req);
				if (res.ok) {
					const cache = await caches.open(CACHE);
					cache.put(req, res.clone());
				}
				return res;
			})()
		);
		return;
	}

	// Other same-origin GET (JS/CSS/fonts/images/manifest): stale-while-revalidate.
	event.respondWith(
		(async () => {
		const cached = await caches.match(req, { cacheName: CACHE });
		if (cached) {
				event.waitUntil(
					fetch(req)
						.then((res) => {
							if (res.ok) {
								caches.open(CACHE).then((cache) => cache.put(req, res.clone()));
							}
						})
						.catch(() => {})
				);
				return cached;
			}
			const res = await fetch(req);
			if (res.ok) {
				const cache = await caches.open(CACHE);
				cache.put(req, res.clone());
			}
			return res;
		})()
	);
});
