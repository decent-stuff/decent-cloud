const CACHE_NAME = 'decent-cloud-v1';
const APP_SHELL = ['/', '/manifest.json', '/favicon.svg', '/offline'];

self.addEventListener('install', (event) => {
	event.waitUntil(
		caches.open(CACHE_NAME).then((cache) => cache.addAll(APP_SHELL))
	);
	self.skipWaiting();
});

self.addEventListener('activate', (event) => {
	event.waitUntil(
		caches.keys().then((keys) =>
			Promise.all(keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k)))
		)
	);
	self.clients.claim();
});

self.addEventListener('fetch', (event) => {
	const req = event.request;

	// Only the app shell (HTML navigations) gets offline fallback. All other
	// requests (API/XHR, static assets) pass straight to the network and are
	// NEVER intercepted. Previously the SW caught every fetch and converted any
	// failure — including transient API hiccups — into an opaque 503, which
	// masked real errors and broke cross-origin API caching (e.g. a cold
	// service-worker registration on a fresh shard origin returned 503 for
	// every API call even though the API itself was healthy).
	if (req.mode !== 'navigate') {
		return;
	}

	event.respondWith(
		caches.match(req).then((cached) => {
			if (cached) return cached;
			return fetch(req).catch(() => caches.match('/offline'));
		})
	);
});
