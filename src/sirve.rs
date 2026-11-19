//! `trompo sirve`: el daemon. Dos puertos:
//!   47420 http   solo la bienvenida (bajar la CA) y redirigir a https
//!   47421 https  la PWA y su API (todo lo de /api pide la cookie del cel,
//!                menos /api/hola y /api/emparejar)

use crate::acciones::{self, Accion};
use crate::estado::{self, Estado};
use crate::{confianza, tema::Tema, web};
use axum::body::Body;
use axum::extract::{Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::Stream;
use mdns_sd::{IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, watch};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::WatchStream;

pub const HTTP: u16 = 47420;
pub const HTTPS: u16 = 47421;
const SERVICIO: &str = "_trompo._tcp.local.";

struct App {
    estado: watch::Receiver<Estado>,
    fuente: Mutex<Option<PathBuf>>,
    miniaturas: Semaphore,
    ca_pem: String,
}

type Shared = Arc<App>;

pub async fn sirve() -> Result<(), String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let host = estado::hostname();

    // un solo trompo por compu
    let _ = std::fs::create_dir_all(crate::runtime_dir());
    let pidfile = crate::runtime_dir().join("sirve.pid");
    if let Some(pid) = std::fs::read_to_string(&pidfile).ok().and_then(|s| s.trim().parse::<u32>().ok())
        && pid != std::process::id()
        && std::path::Path::new(&format!("/proc/{pid}/exe")).read_link().is_ok_and(|e| e.file_name().is_some_and(|n| n.to_string_lossy().starts_with("trompo")))
    {
        return Err(format!("ya hay un trompo sirviendo (pid {pid})"));
    }
    let _ = std::fs::write(&pidfile, std::process::id().to_string());

    let certs = crate::tls::load(&host)?;
    let tls = axum_server::tls_rustls::RustlsConfig::from_pem(certs.cert_pem.into_bytes(), certs.key_pem.into_bytes())
        .await
        .map_err(|e| format!("tls: {e}"))?;

    let mut tema = Tema::new();
    let mut primero = estado::leer();
    primero.looks = estado::looks(&primero.idioma);
    primero.tema = tema.json();
    let (tx, rx) = watch::channel(primero);
    let app = Arc::new(App {
        estado: rx,
        fuente: Mutex::new(tema.fuente.clone()),
        miniaturas: Semaphore::new(3),
        ca_pem: certs.ca_pem,
    });

    // las otras compus con trompo (mDNS) y anunciarse
    let otras: Arc<Mutex<BTreeMap<String, String>>> = Arc::default();
    let mdns = anunciar(&host, otras.clone());

    // el estado, cada segundo; solo se manda si cambió
    {
        let app = app.clone();
        let tls = tls.clone();
        let host = host.clone();
        tokio::spawn(async move {
            let mut tick = 0u64;
            // la posición que se mandó la última vez que se avisó, y cuándo
            let mut avisado: Option<(f64, Instant)> = None;
            let mut ips = crate::tls::local_ips();
            let mut looks = app.estado.borrow().looks.clone();
            let mut look = app.estado.borrow().look.clone();
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                tick += 1;
                let Ok(mut e) = tokio::task::spawn_blocking(estado::leer).await else { continue };
                if tick.is_multiple_of(15) || e.look != look {
                    look = e.look.clone();
                    let lang = e.idioma.clone();
                    looks = tokio::task::spawn_blocking(move || estado::looks(&lang)).await.unwrap_or(looks);
                }
                e.looks = looks.clone();
                if tema.refresh() {
                    *app.fuente.lock().unwrap() = tema.fuente.clone();
                }
                e.tema = tema.json();
                e.otras = otras.lock().unwrap().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                // La posición de la canción la avanza el cel solo: si fue lo único que
                // cambió y va donde se esperaba, se guarda (para quien se conecte
                // después) pero no se le avisa a nadie. ~12 MB/hora → casi nada.
                tx.send_if_modified(|old| {
                    if *old == e {
                        return false;
                    }
                    let avisar = !solo_avanzo(old, &e, avisado);
                    if avisar {
                        avisado = e.musica.as_ref().map(|m| (m.pos, Instant::now()));
                    }
                    *old = e;
                    avisar
                });
                // ¿otra IP (otro Wi-Fi, DHCP)? certificado nuevo para ella
                if tick.is_multiple_of(20) {
                    let ahora = crate::tls::local_ips();
                    if ahora != ips {
                        ips = ahora;
                        if let Ok(c) = crate::tls::load(&host) {
                            let _ = tls.reload_from_pem(c.cert_pem.into_bytes(), c.key_pem.into_bytes()).await;
                        }
                    }
                }
            }
        });
    }

    let api = Router::new()
        .route("/api/estado", get(estado_json))
        .route("/api/eventos", get(eventos))
        .route("/api/accion", post(accion))
        .route("/api/wallpapers", get(wallpapers))
        .route("/api/min", get(miniatura))
        .route("/api/caratula", get(caratula))
        .route("/api/fuente", get(fuente))
        .route("/api/yo", get(yo))
        .route("/api/olvidame", post(olvidame))
        .layer(middleware::from_fn(guardia));

    let https = Router::new()
        .merge(api)
        .route("/api/hola", get(hola))
        .route("/api/emparejar", post(emparejar))
        .route("/trompo-ca.crt", get(ca))
        .fallback(get(estatico))
        .layer(middleware::from_fn(cabeceras))
        .with_state(app.clone());

    let http = Router::new()
        .route("/", get(|| async { web::archivo("bienvenida.html").into_response() }))
        .route("/trompo-ca.crt", get(ca))
        .route("/bienvenida.css", get(|| async { web::archivo("bienvenida.css").into_response() }))
        .route("/icono.svg", get(|| async { web::archivo("icono.svg").into_response() }))
        .fallback(redirect_https)
        .layer(middleware::from_fn(cabeceras))
        .with_state(app.clone());

    eprintln!("trompo: https://{}:{HTTPS} (bienvenida en http://…:{HTTP})", crate::tls::main_ip().map(|i| i.to_string()).unwrap_or(host));
    let a = axum_server::bind_rustls(SocketAddr::from(([0, 0, 0, 0], HTTPS)), tls).serve(https.into_make_service());
    let b = axum_server::bind(SocketAddr::from(([0, 0, 0, 0], HTTP))).serve(http.into_make_service());
    let r = tokio::select! { r = a => r, r = b => r };
    drop(mdns);
    r.map_err(|e| format!("no pude abrir el puerto: {e}"))
}

fn anunciar(host: &str, otras: Arc<Mutex<BTreeMap<String, String>>>) -> Option<ServiceDaemon> {
    let d = ServiceDaemon::new().ok()?;
    let _ = d.disable_interface(vec![IfKind::LoopbackV4, IfKind::LoopbackV6]);
    let ip = crate::tls::main_ip().map(|i| i.to_string()).unwrap_or_default();
    let info = ServiceInfo::new(SERVICIO, host, &format!("{host}.local."), "", HTTPS, &[("ip", ip.as_str()), ("v", env!("CARGO_PKG_VERSION"))][..])
        .ok()?
        .enable_addr_auto();
    d.register(info).ok()?;
    let rx = d.browse(SERVICIO).ok()?;
    let yo = host.to_string();
    std::thread::spawn(move || {
        while let Ok(ev) = rx.recv() {
            let nombre_de = |full: &str| full.strip_suffix(&format!(".{SERVICIO}")).unwrap_or(full).to_string();
            match ev {
                ServiceEvent::ServiceResolved(info) => {
                    let nombre = nombre_de(info.get_fullname());
                    if nombre == yo {
                        continue;
                    }
                    let ip = info
                        .get_property_val_str("ip")
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .or_else(|| info.get_addresses_v4().into_iter().next().map(|i| i.to_string()));
                    if let Some(ip) = ip {
                        otras.lock().unwrap().insert(nombre, format!("https://{ip}:{}/", info.get_port()));
                    }
                }
                ServiceEvent::ServiceRemoved(_, full) => {
                    otras.lock().unwrap().remove(&nombre_de(&full));
                }
                _ => {}
            }
        }
    });
    Some(d)
}

fn cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|c| c.trim().strip_prefix("trompo=").map(str::to_string))
}

/// Un POST que viene de otra página (otro origen) no pasa, aunque traiga la cookie.
fn mismo_origen(headers: &HeaderMap, uri: &Uri) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else { return true };
    // en HTTP/2 no hay `Host`: el nombre viene en la URI (:authority)
    let host = uri.authority().map(|a| a.as_str()).or_else(|| headers.get(header::HOST).and_then(|v| v.to_str().ok())).unwrap_or("");
    origin.strip_prefix("https://") == Some(host)
}

async fn guardia(req: Request, next: Next) -> Response {
    if !mismo_origen(req.headers(), req.uri()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match cookie(req.headers()).and_then(|k| confianza::quien(&k)) {
        Some(_) => next.run(req).await,
        None => (StatusCode::UNAUTHORIZED, Json(json!({ "error": "sin emparejar" }))).into_response(),
    }
}

async fn cabeceras(req: Request, next: Next) -> Response {
    let mut r = next.run(req).await;
    let h = r.headers_mut();
    h.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    h.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    h.insert("x-frame-options", HeaderValue::from_static("DENY"));
    r
}

async fn redirect_https(headers: HeaderMap, req: Request) -> Response {
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("");
    let host = host.rsplit_once(':').map(|(h, _)| h).unwrap_or(host);
    Redirect::temporary(&format!("https://{host}:{HTTPS}{}", req.uri().path())).into_response()
}

async fn estatico(req: Request) -> Response {
    let p = req.uri().path().trim_start_matches('/');
    web::archivo(if p.is_empty() { "index.html" } else { p }).into_response()
}

async fn ca(State(app): State<Shared>) -> Response {
    (
        [(header::CONTENT_TYPE, "application/x-x509-ca-cert"), (header::CONTENT_DISPOSITION, "attachment; filename=\"trompo-ca.crt\"")],
        app.ca_pem.clone(),
    )
        .into_response()
}

async fn hola() -> Response {
    ([(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], Json(json!({ "trompo": env!("CARGO_PKG_VERSION"), "maquina": estado::hostname() }))).into_response()
}

#[derive(Deserialize)]
struct Emparejar {
    codigo: String,
    nombre: String,
}

async fn emparejar(headers: HeaderMap, uri: Uri, Json(e): Json<Emparejar>) -> Response {
    if !mismo_origen(&headers, &uri) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(llave) = confianza::canjear(e.codigo.trim(), &e.nombre) else {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "código vencido o ya usado" }))).into_response();
    };
    let lang = estado::idioma();
    let (t, b) = if lang == "es" { ("trompo", format!("{} ya maneja esta compu", e.nombre)) } else { ("trompo", format!("{} can now control this computer", e.nombre)) };
    let _ = std::process::Command::new("notify-send").args(["-a", "trompo", "-i", "phone", t, &b]).spawn().map(|mut c| std::thread::spawn(move || c.wait()));
    let c = format!("trompo={llave}; Path=/; Max-Age=315360000; HttpOnly; Secure; SameSite=Strict");
    ([(header::SET_COOKIE, c)], Json(json!({ "ok": true }))).into_response()
}

async fn estado_json(State(app): State<Shared>) -> Json<Estado> {
    Json(app.estado.borrow().clone())
}

async fn eventos(State(app): State<Shared>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let s = WatchStream::new(app.estado.clone()).map(|e| Ok(Event::default().json_data(&e).unwrap_or_default()));
    Sse::new(s).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

async fn accion(Json(a): Json<Accion>) -> Response {
    match tokio::task::spawn_blocking(move || acciones::hacer(a)).await {
        Ok(Ok(())) => Json(json!({ "ok": true })).into_response(),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn wallpapers() -> Response {
    let list = tokio::task::spawn_blocking(acciones::wallpapers).await.unwrap_or_default();
    let list: Vec<_> = list.into_iter().map(|f| json!({ "archivo": f, "video": acciones::es_video(std::path::Path::new(&f)) })).collect();
    Json(list).into_response()
}

#[derive(Deserialize)]
struct Min {
    f: String,
    #[serde(default)]
    a: bool,
}

async fn miniatura(State(app): State<Shared>, Query(q): Query<Min>) -> Response {
    let Some(f) = acciones::wallpaper_seguro(&q.f) else { return StatusCode::NOT_FOUND.into_response() };
    let Ok(_permiso) = app.miniaturas.acquire().await else { return StatusCode::SERVICE_UNAVAILABLE.into_response() };
    match tokio::task::spawn_blocking(move || acciones::miniatura(&f, q.a)).await.ok().flatten().and_then(|p| std::fs::read(p).ok()) {
        Some(b) => ([(header::CONTENT_TYPE, "image/jpeg"), (header::CACHE_CONTROL, "private, max-age=604800")], b).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// La carátula local (file://) del reproductor, solo si de verdad es imagen.
async fn caratula() -> Response {
    let r = tokio::task::spawn_blocking(|| {
        let url = estado::art_url()?;
        let path = url.strip_prefix("file://")?;
        let path = path.replace("%20", " ");
        let b = std::fs::read(path).ok()?;
        let tipo = match b.get(..4)? {
            [0xff, 0xd8, ..] => "image/jpeg",
            [0x89, b'P', b'N', b'G'] => "image/png",
            [b'R', b'I', b'F', b'F'] => "image/webp",
            _ => return None,
        };
        Some((tipo, b))
    })
    .await
    .ok()
    .flatten();
    match r {
        Some((t, b)) => ([(header::CONTENT_TYPE, t), (header::CACHE_CONTROL, "private, max-age=86400")], b).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn fuente(State(app): State<Shared>) -> Response {
    let p = app.fuente.lock().unwrap().clone();
    let p_otf = p.as_ref().is_some_and(|p| p.extension().is_some_and(|e| e == "otf"));
    match p.and_then(|p| std::fs::read(p).ok()) {
        Some(b) => ([(header::CONTENT_TYPE, if p_otf { "font/otf" } else { "font/ttf" }), (header::CACHE_CONTROL, "private, max-age=31536000, immutable")], Body::from(b)).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn yo(headers: HeaderMap) -> Response {
    let nombre = cookie(&headers).and_then(|k| confianza::quien(&k)).unwrap_or_default();
    Json(json!({ "nombre": nombre })).into_response()
}

async fn olvidame(headers: HeaderMap) -> Response {
    if let Some(k) = cookie(&headers) {
        confianza::olvidar_llave(&k);
    }
    ([(header::SET_COOKIE, "trompo=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Strict")], Json(json!({ "ok": true }))).into_response()
}

/// ¿Lo único distinto es la posición de una canción que sigue sonando, y va
/// donde el cel ya la tiene (lo avisado + lo que ha pasado, ±2 s)?
fn solo_avanzo(old: &Estado, new: &Estado, avisado: Option<(f64, Instant)>) -> bool {
    let (Some(a), Some(b), Some((pos, cuando))) = (&old.musica, &new.musica, avisado) else { return false };
    if b.estado != "Playing" || a.estado != "Playing" {
        return false;
    }
    let mut sin_pos = old.clone();
    if let Some(m) = sin_pos.musica.as_mut() {
        m.pos = b.pos;
    }
    sin_pos == *new && (b.pos - (pos + cuando.elapsed().as_secs_f64())).abs() < 2.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::estado::Musica;

    fn con(pos: f64, estado: &str) -> Estado {
        Estado { musica: Some(Musica { estado: estado.into(), titulo: "t".into(), largo: 200.0, pos, ..Default::default() }), ..Default::default() }
    }

    #[test]
    fn solo_avanza_la_cancion() {
        let hace = Instant::now() - Duration::from_secs(5);
        let avisado = Some((10.0, hace));
        // sonando y va donde se esperaba: no se avisa
        assert!(solo_avanzo(&con(14.0, "Playing"), &con(15.0, "Playing"), avisado));
        // brincó (la adelantaron): se avisa
        assert!(!solo_avanzo(&con(14.0, "Playing"), &con(60.0, "Playing"), avisado));
        // pausa: se avisa
        assert!(!solo_avanzo(&con(14.0, "Playing"), &con(15.0, "Paused"), avisado));
        // otra canción: se avisa
        let mut otra = con(15.0, "Playing");
        otra.musica.as_mut().unwrap().titulo = "otra".into();
        assert!(!solo_avanzo(&con(14.0, "Playing"), &otra, avisado));
        // cambió el volumen: se avisa
        let mut vol = con(15.0, "Playing");
        vol.volumen = Some((40, false));
        assert!(!solo_avanzo(&con(14.0, "Playing"), &vol, avisado));
    }
}
