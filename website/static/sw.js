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
	event.respondWith(
		caches.match(event.request).then((cached) => {
			if (cached) return cached;
			return fetch(event.request).catch(() => {
				if (event.request.mode === 'navigate') {
					return caches.match('/offline');
				}
				return new Response('', { status: 503 });
			});
		})
	);
});
