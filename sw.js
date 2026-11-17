// The wasm/js files are not hashed (`filehash = false` in Trunk.toml), while `index.html`
// pins them with `integrity="sha384-..."`. Serving them cache-first made a stale cached
// wasm/js fail that check after a deploy, leaving the page on "Loading..." forever.
// So: network first (revalidated), the cache is only the offline fallback.
var cacheName = 'prono-pwa-v2';
var filesToCache = [
  './',
  './index.html',
  './prono-app.js',
  './prono-app_bg.wasm',
];

/* Start the service worker, cache the app's content and take over right away */
self.addEventListener('install', function (e) {
  self.skipWaiting();
  e.waitUntil(
    caches.open(cacheName).then(function (cache) {
      return cache.addAll(filesToCache.map(function (url) {
        return new Request(url, { cache: 'reload' });
      }));
    })
  );
});

/* Drop the caches of older versions (e.g. the cache-first 'egui-template-pwa') */
self.addEventListener('activate', function (e) {
  e.waitUntil(
    caches.keys()
      .then(function (names) {
        return Promise.all(names.filter(function (name) {
          return name !== cacheName;
        }).map(function (name) {
          return caches.delete(name);
        }));
      })
      .then(function () {
        return self.clients.claim();
      })
  );
});

/* Always try the network, refresh the cache with the answer, and use the cache when offline */
self.addEventListener('fetch', function (e) {
  if (e.request.method !== 'GET' || new URL(e.request.url).origin !== self.location.origin) {
    return;
  }
  e.respondWith(
    fetch(e.request, { cache: 'no-cache' })
      .then(function (response) {
        if (response.ok) {
          var copy = response.clone();
          caches.open(cacheName).then(function (cache) {
            return cache.put(e.request, copy);
          });
        }
        return response;
      })
      .catch(function () {
        return caches.match(e.request, { ignoreSearch: true });
      })
  );
});
