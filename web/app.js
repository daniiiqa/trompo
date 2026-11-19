// trompo: la PWA. Recibe el estado de la compu por SSE (/api/eventos) y
// le manda acciones a /api/accion. Se pinta con el look que tenga la compu.
"use strict";

const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];

// ---------- textos ----------
const T = {
  es: {
    conectando: "Conectando con la compu…", sin_conexion: "Sin conexión con la compu. Reintentando…",
    emparejar_intro: "Ponle nombre a este cel para que la compu sepa quién la maneja.",
    nombre_cel: "Nombre del cel", emparejar: "Emparejar",
    sin_codigo: "Este cel no está emparejado. En la compu corre:",
    codigo_malo: "El código se venció o ya se usó. Corre `trompo` otra vez en la compu.",
    look_ahora: "LOOK", nada_sonando: "Nada sonando", volumen: "Volumen", brillo: "Brillo",
    bloquear: "Bloquear", suspender: "Suspender", toca_otra: "toca otra vez",
    looks: "Looks", wallpapers: "Wallpapers", inicio: "Inicio",
    machin_apagado: "apagado", machin_principal: "usando ", machin_presta: "prestada a ",
    machin_buscando: "buscando…", del_wallpaper: "del wallpaper",
    cambiando_look: "Cambiando a ", poniendo_wall: "Poniendo el wallpaper…",
    error: "No se pudo: ",
  },
  en: {
    conectando: "Connecting to the computer…", sin_conexion: "Lost the computer. Retrying…",
    emparejar_intro: "Name this phone so the computer knows who's driving.",
    nombre_cel: "Phone name", emparejar: "Pair",
    sin_codigo: "This phone isn't paired. On the computer, run:",
    codigo_malo: "The code expired or was already used. Run `trompo` again on the computer.",
    look_ahora: "LOOK", nada_sonando: "Nothing playing", volumen: "Volume", brillo: "Brightness",
    bloquear: "Lock", suspender: "Sleep", toca_otra: "tap again",
    looks: "Looks", wallpapers: "Wallpapers", inicio: "Home",
    machin_apagado: "off", machin_principal: "using ", machin_presta: "lent to ",
    machin_buscando: "looking…", del_wallpaper: "from wallpaper",
    cambiando_look: "Switching to ", poniendo_wall: "Setting the wallpaper…",
    error: "Couldn't: ",
  },
};
let lang = (navigator.language || "").startsWith("es") ? "es" : "en";
const t = (k) => T[lang][k] ?? k;
function textos() {
  document.documentElement.lang = lang;
  for (const el of $$("[data-t]")) el.textContent = t(el.dataset.t);
}

// ---------- íconos (trazos propios, se pintan con currentColor) ----------
const P = {
  prev: '<path d="M6 5v14"/><path d="M19 5 9 12l10 7z" fill="currentColor"/>',
  next: '<path d="M18 5v14"/><path d="m5 5 10 7-10 7z" fill="currentColor"/>',
  play: '<path d="M7 4.5v15l12.5-7.5z" fill="currentColor" stroke-linejoin="round"/>',
  pause: '<rect x="6" y="4.5" width="4" height="15" rx="1" fill="currentColor"/><rect x="14" y="4.5" width="4" height="15" rx="1" fill="currentColor"/>',
  vol: '<path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z" fill="currentColor"/><path d="M15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11"/>',
  mudo: '<path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z" fill="currentColor"/><path d="m16 9.5 5 5m0-5-5 5"/>',
  sol: '<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2.5M12 19v2.5M2.5 12H5M19 12h2.5M5.3 5.3l1.8 1.8M16.9 16.9l1.8 1.8M5.3 18.7l1.8-1.8M16.9 7.1l1.8-1.8"/>',
  monitor: '<rect x="2.5" y="4" width="13" height="10" rx="1.5"/><rect x="13" y="8" width="8.5" height="7" rx="1.5" fill="var(--card)"/><path d="M6 18h6M9 14v4"/>',
  candado: '<rect x="5" y="10.5" width="14" height="10" rx="2"/><path d="M8 10.5V7.5a4 4 0 0 1 8 0v3"/>',
  luna: '<path d="M19.5 14.5A8 8 0 0 1 9.5 4.5a8 8 0 1 0 10 10z"/>',
  casa: '<path d="M3.5 11 12 4l8.5 7"/><path d="M6 9.5V20h12V9.5"/>',
  paleta: '<path d="M12 3a9 9 0 1 0 0 18c1.2 0 1.8-.8 1.8-1.8 0-1.3-1-1.6-1-2.7 0-1 .8-1.8 1.8-1.8H17a4 4 0 0 0 4-4C21 6.5 17 3 12 3z"/><circle cx="7.5" cy="11" r="1.2" fill="currentColor"/><circle cx="10.5" cy="7" r="1.2" fill="currentColor"/><circle cx="15" cy="7.5" r="1.2" fill="currentColor"/>',
  imagen: '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="9.5" r="1.8"/><path d="m3.5 18 5.5-5.5 4 4 2.5-2.5 5 4.5"/>',
  nota: '<path d="M9 18V5.5l11-2v12.5"/><circle cx="6.5" cy="18" r="2.5" fill="currentColor"/><circle cx="17.5" cy="16" r="2.5" fill="currentColor"/>',
  video: '<path d="M7 4.5v15l12.5-7.5z" fill="currentColor"/>',
};
const svg = (n) => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${P[n] || ""}</svg>`;
function icono(el, n) {
  if (el.dataset.icoActual === n) return;
  el.dataset.icoActual = n;
  el.innerHTML = svg(n);
}
for (const el of $$("[data-ico]")) icono(el, el.dataset.ico);

// ---------- avisos ----------
let avisoTimer;
function aviso(txt) {
  const a = $("#aviso");
  a.textContent = txt;
  a.classList.add("ver");
  clearTimeout(avisoTimer);
  avisoTimer = setTimeout(() => a.classList.remove("ver"), 2400);
}

// ---------- hablar con la compu ----------
async function accion(cuerpo) {
  try {
    const r = await fetch("/api/accion", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(cuerpo) });
    if (r.status === 401) return location.reload();
    if (!r.ok) {
      const e = await r.json().catch(() => ({}));
      aviso(t("error") + (e.error || r.status));
      return false;
    }
    navigator.vibrate?.(8);
    return true;
  } catch {
    aviso(t("sin_conexion"));
    return false;
  }
}

// ---------- el tema ----------
let temaActual = "";
let toque = { x: "50%", y: "50%" };
document.addEventListener("pointerdown", (e) => { toque = { x: e.clientX + "px", y: e.clientY + "px" }; }, true);

function luminancia(hex) {
  const n = parseInt(hex.slice(1), 16);
  const c = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; });
  return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
}

let fuenteCargada = "";
function aplicarTema(tema) {
  const r = document.documentElement;
  const s = r.style;
  for (const k of ["base", "primary", "accent", "fg", "muted", "border", "hl", "red", "green"]) s.setProperty("--" + k, tema[k]);
  s.setProperty("--radius", tema.radius + "px");
  s.setProperty("--bw", tema.borderWidth + "px");
  s.setProperty("--bs", tema.borderStyle);
  s.setProperty("--case", tema.case);
  r.dataset.bar = tema.bar;
  r.dataset.select = tema.select;
  r.dataset.radius = tema.radius;
  if (luminancia(tema.base) > 0.4) r.dataset.claro = ""; else delete r.dataset.claro;
  $('meta[name="theme-color"]').content = tema.base;
  if (tema.fuente && tema.fuente !== fuenteCargada) {
    fuenteCargada = tema.fuente;
    const f = new FontFace("trompo-look", `url(${tema.fuente}) format("${tema.fuenteFormato}")`);
    f.load().then((ok) => {
      document.fonts.add(ok);
      s.setProperty("--font", `"trompo-look", ui-sans-serif, system-ui, sans-serif`);
    }).catch(() => {});
  } else if (!tema.fuente) {
    s.removeProperty("--font");
  }
}

function tema(tema) {
  const k = JSON.stringify(tema);
  if (k === temaActual) return;
  const primera = !temaActual;
  temaActual = k;
  if (primera || !document.startViewTransition || matchMedia("(prefers-reduced-motion: reduce)").matches) {
    aplicarTema(tema);
    return;
  }
  // el iris sale de donde tocaste (o del centro si cambió desde la compu)
  document.documentElement.style.setProperty("--ix", toque.x);
  document.documentElement.style.setProperty("--iy", toque.y);
  document.startViewTransition(() => aplicarTema(tema));
  toque = { x: "50%", y: "50%" };
}

// ---------- pintar el estado ----------
let E = null;
let arrastrando = { volumen: false, brillo: false };
let esperandoLook = "";
let esperandoWall = "";

const mmss = (s) => { s = Math.max(0, Math.floor(s || 0)); return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`; };

function maquinas(e) {
  const cont = $("#maquinas");
  const html = [`<span class="maquina yo">${esc(e.maquina)}</span>`]
    .concat((e.otras || []).map(([n, url]) => `<a class="maquina" href="${esc(url)}">${esc(n)}</a>`)).join("");
  if (cont.innerHTML !== html) cont.innerHTML = html;
  document.title = `trompo · ${e.maquina}`;
}

const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

let musicaBase = { pos: 0, largo: 0, cuando: 0, sonando: false };
function musica(m) {
  $("#musica").style.display = m ? "" : "none";
  $("#nada-sonando").style.display = m ? "none" : "";
  if (!m) { musicaBase.sonando = false; return; }
  $("#titulo").textContent = m.titulo || "—";
  $("#artista").textContent = [m.artista, m.album].filter(Boolean).join(" · ") || m.reproductor;
  const img = $("#caratula-img");
  const src = m.caratula || "";
  if (img.getAttribute("src") !== src) { if (src) img.src = src; else img.removeAttribute("src"); }
  img.onerror = () => img.removeAttribute("src");
  icono($("#play"), m.estado === "Playing" ? "pause" : "play");
  musicaBase = { pos: m.pos, largo: m.largo, cuando: performance.now(), sonando: m.estado === "Playing" };
  progreso();
}
function progreso() {
  const b = musicaBase;
  const pos = b.sonando ? Math.min(b.largo || Infinity, b.pos + (performance.now() - b.cuando) / 1000) : b.pos;
  $("#progreso i").style.width = b.largo ? `${(pos / b.largo) * 100}%` : "0";
  $("#t-pos").textContent = mmss(pos);
  $("#t-largo").textContent = b.largo ? mmss(b.largo) : "";
}
setInterval(() => { if (musicaBase.sonando && !document.hidden) progreso(); }, 500);

function deslizador(id, valor) {
  const tarjeta = $(`#tarjeta-${id}`);
  tarjeta.style.display = valor == null ? "none" : "";
  if (valor == null || arrastrando[id]) return;
  const r = $(`#${id}`);
  r.value = valor;
  r.style.setProperty("--p", valor + "%");
  $(`#${id}-n`).textContent = valor;
}

function machin(e) {
  const b = $("#machin");
  b.style.display = e.machin == null ? "none" : "";
  b.classList.toggle("prendido", !!e.machin);
  b.querySelector("small").textContent =
    e.machin === "principal" ? t("machin_principal") + e.machin_otra
    : e.machin === "presta" ? t("machin_presta") + e.machin_otra
    : b.dataset.buscando ? t("machin_buscando") : t("machin_apagado");
}

function looks(e) {
  const cont = $("#looks");
  const firma = JSON.stringify([e.looks, e.look, esperandoLook, lang]);
  if (cont.dataset.firma === firma) return;
  cont.dataset.firma = firma;
  cont.innerHTML = e.looks.map((l) => {
    const muestra = l.muestra.length
      ? l.muestra.map((c) => `<i style="background:${esc(c)}"></i>`).join("")
      : `<em>${t("del_wallpaper")}</em>`;
    const cls = ["look", l.nombre === e.look ? "actual" : "", l.nombre === esperandoLook ? "esperando" : ""].join(" ");
    return `<button class="${cls}" data-look="${esc(l.nombre)}"><b>${esc(l.nombre)}</b><span>${esc(l.about)}</span><div class="muestra">${muestra}</div></button>`;
  }).join("");
}

function render(e) {
  const antes = E;
  E = e;
  if (e.idioma && e.idioma !== lang) { lang = e.idioma; textos(); }
  if (e.tema) tema(e.tema);
  maquinas(e);
  musica(e.musica);
  if (e.volumen) {
    deslizador("volumen", e.volumen[0]);
    $("#tarjeta-volumen").classList.toggle("mudo", e.volumen[1]);
    icono($("#mudo-ico"), e.volumen[1] ? "mudo" : "vol");
  } else deslizador("volumen", null);
  deslizador("brillo", e.brillo);
  if (esperandoLook && e.look === esperandoLook) esperandoLook = "";
  looks(e);
  machin(e);
  $("#ahora-look").textContent = e.look;
  if (!antes || antes.wallpaper !== e.wallpaper) {
    const img = $("#ahora-img");
    img.classList.remove("lista");
    img.onload = () => img.classList.add("lista");
    if (e.wallpaper) img.src = `/api/min?a=true&f=${encodeURIComponent(e.wallpaper)}`;
    esperandoWall = "";
    marcarWall();
  }
}

// ---------- wallpapers (se cargan al abrir la pestaña) ----------
let wallsCargados = false;
async function cargarWalls() {
  if (wallsCargados) return;
  wallsCargados = true;
  const r = await fetch("/api/wallpapers").catch(() => null);
  if (!r?.ok) { wallsCargados = false; return; }
  const lista = await r.json();
  const cont = $("#wallpapers");
  cont.innerHTML = lista.map((w) =>
    `<button class="wall" data-wall="${esc(w.archivo)}"><img loading="lazy" alt="" src="/api/min?f=${encodeURIComponent(w.archivo)}" onload="this.classList.add('lista')">${w.video ? `<span class="video">${svg("video")}</span>` : ""}</button>`
  ).join("");
  marcarWall();
}
function marcarWall() {
  for (const b of $$(".wall")) {
    b.classList.toggle("actual", !!E && b.dataset.wall === E.wallpaper);
    b.classList.toggle("esperando", b.dataset.wall === esperandoWall);
  }
}

// ---------- eventos de la página ----------
for (const b of $$("nav button")) {
  b.onclick = () => {
    for (const x of $$("nav button")) x.classList.toggle("activa", x === b);
    for (const v of $$(".vista")) v.classList.toggle("activa", v.id === "vista-" + b.dataset.vista);
    if (b.dataset.vista === "wallpapers") cargarWalls();
    try { localStorage.setItem("trompo.vista", b.dataset.vista); } catch {}
    scrollTo({ top: 0 });
  };
}
try {
  const v = localStorage.getItem("trompo.vista");
  if (v) $(`nav button[data-vista="${v}"]`)?.click();
} catch {}
if (!$(".vista.activa")) $("#vista-inicio").classList.add("activa");

for (const b of $$("[data-musica]")) b.onclick = () => accion({ que: "musica", boton: b.dataset.musica });
$("#ahora").onclick = () => $(`nav button[data-vista="looks"]`).click();
$("#mudo").onclick = () => accion({ que: "mudo" });

function engancharDeslizador(id, que) {
  const r = $(`#${id}`);
  let timer;
  const mandar = () => accion({ que, nivel: +r.value });
  r.addEventListener("input", () => {
    arrastrando[id] = true;
    r.style.setProperty("--p", r.value + "%");
    $(`#${id}-n`).textContent = r.value;
    clearTimeout(timer);
    timer = setTimeout(mandar, 90);
  });
  r.addEventListener("change", () => {
    clearTimeout(timer);
    mandar().finally(() => setTimeout(() => { arrastrando[id] = false; }, 600));
  });
}
engancharDeslizador("volumen", "volumen");
engancharDeslizador("brillo", "brillo");

$("#machin").onclick = async () => {
  const b = $("#machin");
  b.dataset.buscando = E?.machin ? "" : "1";
  if (E) machin(E);
  await accion({ que: "machin" });
  setTimeout(() => { delete b.dataset.buscando; if (E) machin(E); }, 8000);
};

// las peligrosas piden un segundo toque
for (const b of $$("[data-confirma]")) {
  let armado = 0;
  b.onclick = () => {
    if (Date.now() - armado < 3000) {
      armado = 0;
      b.classList.remove("armado");
      b.querySelector("small").textContent = "";
      accion({ que: b.id });
      return;
    }
    armado = Date.now();
    b.classList.add("armado");
    b.querySelector("small").textContent = t("toca_otra");
    navigator.vibrate?.(15);
    setTimeout(() => { if (Date.now() - armado >= 2900) { b.classList.remove("armado"); b.querySelector("small").textContent = ""; } }, 3000);
  };
}

$("#looks").onclick = async (ev) => {
  const b = ev.target.closest("[data-look]");
  if (!b || !E || b.dataset.look === E.look) return;
  esperandoLook = b.dataset.look;
  looks(E);
  aviso(t("cambiando_look") + b.dataset.look);
  if (!(await accion({ que: "look", nombre: b.dataset.look }))) { esperandoLook = ""; looks(E); }
  setTimeout(() => { if (esperandoLook) { esperandoLook = ""; looks(E); } }, 20000);
};

$("#wallpapers").onclick = async (ev) => {
  const b = ev.target.closest("[data-wall]");
  if (!b || (E && b.dataset.wall === E.wallpaper)) return;
  esperandoWall = b.dataset.wall;
  marcarWall();
  aviso(t("poniendo_wall"));
  if (!(await accion({ que: "wallpaper", archivo: b.dataset.wall }))) { esperandoWall = ""; marcarWall(); }
  setTimeout(() => { esperandoWall = ""; marcarWall(); }, 15000);
};

// ---------- arrancar ----------
function pantalla(p) { document.body.dataset.pantalla = p; }

function conectar() {
  const es = new EventSource("/api/eventos");
  let vivo = false;
  es.onmessage = (m) => {
    vivo = true;
    try { render(JSON.parse(m.data)); } catch (err) { console.error(err); }
    pantalla("app");
  };
  es.onerror = async () => {
    // ¿se cayó la red o nos olvidaron? (EventSource no dice el código)
    if (!vivo) {
      const r = await fetch("/api/yo", { cache: "no-store" }).catch(() => null);
      if (r && r.status === 401) { es.close(); return emparejar(); }
    }
    if (es.readyState === EventSource.CLOSED) setTimeout(conectar, 2000);
    if (!E) { $("#cargando-texto").textContent = t("sin_conexion"); pantalla("cargando"); }
  };
}

function codigoDelHash() {
  const m = /[#&]c=([0-9a-f]+)/.exec(location.hash);
  return m ? m[1] : "";
}

function nombreSugerido() {
  const ua = navigator.userAgentData?.platform || "";
  const m = /Android [\d.]+; ([^;)]+)/.exec(navigator.userAgent);
  const modelo = m && m[1] !== "K" ? m[1].trim() : "";
  return modelo || (ua ? `cel (${ua})` : "cel");
}

function emparejar() {
  const codigo = codigoDelHash();
  $("#emparejar-con-codigo").style.display = codigo ? "" : "none";
  $("#emparejar-sin-codigo").style.display = codigo ? "none" : "";
  $("#nombre").value = nombreSugerido();
  pantalla("emparejar");
  $("#emparejar-boton").onclick = async () => {
    $("#emparejar-error").textContent = "";
    const r = await fetch("/api/emparejar", {
      method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify({ codigo, nombre: $("#nombre").value.trim() || "cel" }),
    }).catch(() => null);
    if (r?.ok) {
      history.replaceState(null, "", "/");
      pantalla("cargando");
      conectar();
    } else {
      $("#emparejar-error").textContent = r ? t("codigo_malo") : t("sin_conexion");
    }
  };
}

async function arrancar() {
  textos();
  $("#cargando-texto").textContent = t("conectando");
  const r = await fetch("/api/yo", { cache: "no-store" }).catch(() => null);
  if (!r) { $("#cargando-texto").textContent = t("sin_conexion"); setTimeout(arrancar, 3000); return; }
  if (r.status === 401) return emparejar();
  if (location.hash) history.replaceState(null, "", "/");
  conectar();
}

if ("serviceWorker" in navigator) navigator.serviceWorker.register("/sw.js").catch(() => {});
arrancar();
