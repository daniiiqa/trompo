//! La PWA va dentro del binario (web/): un solo archivo que instalar.

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

macro_rules! archivos {
    ($($n:literal => $t:literal),* $(,)?) => {
        const ARCHIVOS: &[(&str, &str, &[u8])] = &[$(($n, $t, include_bytes!(concat!("../web/", $n)))),*];
    };
}

archivos! {
    "index.html" => "text/html; charset=utf-8",
    "bienvenida.html" => "text/html; charset=utf-8",
    "bienvenida.css" => "text/css; charset=utf-8",
    "app.css" => "text/css; charset=utf-8",
    "app.js" => "text/javascript; charset=utf-8",
    "sw.js" => "text/javascript; charset=utf-8",
    "manifest.webmanifest" => "application/manifest+json",
    "icono.svg" => "image/svg+xml",
    "icono-192.png" => "image/png",
    "icono-512.png" => "image/png",
    "icono-maskable.png" => "image/png",
}

pub fn archivo(nombre: &str) -> Response {
    match ARCHIVOS.iter().find(|(n, _, _)| *n == nombre) {
        Some((_, tipo, bytes)) => {
            // el html y el service worker siempre frescos; lo demás lo versiona el sw
            let cache = if nombre.ends_with(".html") || nombre == "sw.js" { "no-cache" } else { "public, max-age=3600" };
            ([(header::CONTENT_TYPE, *tipo), (header::CACHE_CONTROL, cache)], *bytes).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
