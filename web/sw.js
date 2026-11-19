// trompo: el service worker. Guarda la cáscara de la app (para que abra al
// instante y se pueda instalar); lo de /api siempre va a la compu.
const CACHE = "trompo-v1";
const CASCARA = ["/", "/app.css", "/app.js", "/manifest.webmanifest", "/icono.svg", "/icono-192.png", "/icono-512.png"];

self.addEventListener("install", (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(CASCARA)).then(() => self.skipWaiting()));
});

self.addEventListener("activate", (e) => {
  e.waitUntil(
    caches.keys().then((ks) => Promise.all(ks.filter((k) => k !== CACHE).map((k) => caches.delete(k)))).then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (e) => {
  const url = new URL(e.request.url);
  if (e.request.method !== "GET" || url.origin !== location.origin || url.pathname.startsWith("/api/")) return;
  // la red primero (así siempre sale la versión nueva); sin red, lo guardado
  const clave = e.request.mode === "navigate" ? "/" : e.request;
  e.respondWith(
    fetch(e.request)
      .then((r) => {
        if (r.ok) { const copia = r.clone(); caches.open(CACHE).then((c) => c.put(clave, copia)); }
        return r;
      })
      .catch(() => caches.match(clave))
  );
});
