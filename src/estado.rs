//! Lo que el cel ve de la compu, armado cada segundo con lo mismo que usa
//! discada (playerctl, wpctl, brightnessctl, hypr/looks, machín). El cel lo
//! recibe por SSE solo cuando cambia.

use serde::Serialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Clone, PartialEq, Default)]
pub struct Musica {
    pub estado: String, // Playing | Paused | Stopped
    pub titulo: String,
    pub artista: String,
    pub album: String,
    pub largo: f64, // segundos
    pub pos: f64,
    pub caratula: Option<String>, // url (la de la compu pasa por /api/caratula)
    pub reproductor: String,
}

#[derive(Serialize, Clone, PartialEq)]
pub struct LookInfo {
    pub nombre: String,
    pub about: String,
    /// colores fijos del look (los de paleta propia), para la muestra
    pub muestra: Vec<String>,
}

#[derive(Serialize, Clone, PartialEq, Default)]
pub struct Estado {
    pub maquina: String,
    pub idioma: String,
    pub musica: Option<Musica>,
    pub volumen: Option<(u32, bool)>,
    pub brillo: Option<u32>,
    pub look: String,
    pub looks: Vec<LookInfo>,
    pub wallpaper: String,
    /// None: machín no está; Some("") apagado; "principal" / "presta"
    pub machin: Option<String>,
    pub machin_otra: String,
    pub tema: Value,
    pub otras: Vec<(String, String)>,
}

pub fn cfg() -> PathBuf {
    crate::home().join(".config")
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(cmd).args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

pub fn idioma() -> String {
    let conf = std::fs::read_to_string(cfg().join("hypr/discada.conf")).unwrap_or_default();
    let es = conf.lines().any(|l| l.trim_start().starts_with("LANGUAGE=es"));
    if es { "es".into() } else { "en".into() }
}

/// La URL "real" de la carátula que reporta el reproductor (file:// o http).
pub fn art_url() -> Option<String> {
    run("playerctl", &["metadata", "mpris:artUrl"]).filter(|s| !s.is_empty())
}

fn musica() -> Option<Musica> {
    let f = "{{status}}\x1f{{title}}\x1f{{artist}}\x1f{{album}}\x1f{{mpris:length}}\x1f{{position}}\x1f{{mpris:artUrl}}\x1f{{playerName}}";
    let out = run("playerctl", &["metadata", "--format", f])?;
    let p: Vec<&str> = out.split('\x1f').collect();
    if p.len() < 8 {
        return None;
    }
    let us = |s: &str| s.parse::<f64>().map(|x| x / 1e6).unwrap_or(0.0);
    let art = p[6];
    let caratula = if art.starts_with("file://") {
        // la versión cambia con la canción: el cel no se queda con la vieja
        let v: u32 = art.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
        Some(format!("/api/caratula?v={v:x}"))
    } else if art.starts_with("https://") {
        Some(art.to_string())
    } else {
        None
    };
    Some(Musica {
        estado: p[0].into(),
        titulo: p[1].into(),
        artista: p[2].into(),
        album: p[3].into(),
        largo: us(p[4]),
        pos: us(p[5]),
        caratula,
        reproductor: p[7].into(),
    })
}

fn volumen() -> Option<(u32, bool)> {
    // "Volume: 0.45 [MUTED]"
    let out = run("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
    let v: f64 = out.split_whitespace().nth(1)?.parse().ok()?;
    Some(((v * 100.0).round() as u32, out.contains("MUTED")))
}

fn brillo() -> Option<u32> {
    std::fs::read_dir("/sys/class/backlight").ok()?.next()?.ok()?;
    let out = run("brightnessctl", &["-m", "-c", "backlight"])?;
    out.split(',').nth(3)?.trim_end_matches('%').parse().ok()
}

fn looks_dir() -> PathBuf {
    cfg().join("hypr/looks")
}

pub fn look_actual() -> String {
    std::fs::read_to_string(looks_dir().join(".current")).unwrap_or_default().trim().to_string()
}

fn look_info(nombre: &str, lang: &str) -> LookInfo {
    let t: toml::Table = std::fs::read_to_string(looks_dir().join(nombre).join("look.toml"))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();
    let about = t.get("about").and_then(|a| a.get(lang)).and_then(|v| v.as_str()).unwrap_or("").to_string();
    // los de paleta fija traen background/foreground/color* en [colors]
    let mut muestra = Vec::new();
    if let Some(c) = t.get("colors").and_then(|v| v.as_table()) {
        for k in ["background", "color4", "color12", "color11", "color9", "foreground"] {
            if let Some(s) = c.get(k).and_then(|v| v.as_str()).filter(|s| s.starts_with('#')) {
                muestra.push(s.to_string());
            }
        }
    }
    LookInfo { nombre: nombre.into(), about, muestra }
}

pub fn looks(lang: &str) -> Vec<LookInfo> {
    let lena = crate::home().join(".local/bin/lena");
    let names: Vec<String> = match run(lena.to_str().unwrap_or("lena"), &["list"]) {
        Some(s) => s.lines().map(str::to_string).filter(|l| !l.is_empty()).collect(),
        None => {
            let mut v: Vec<String> = std::fs::read_dir(looks_dir())
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| e.path().join("look.toml").exists())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            v.sort();
            v
        }
    };
    names.iter().map(|n| look_info(n, lang)).collect()
}

pub fn wallpapers_dir() -> PathBuf {
    run("xdg-user-dir", &["PICTURES"]).map(PathBuf::from).unwrap_or_else(|| crate::home().join("Pictures")).join("wallpapers")
}

/// El wallpaper de ahora (ruta relativa a la carpeta de wallpapers): el video
/// de livewallpaper.conf si hay uno, si no la imagen que reporta awww.
fn wallpaper() -> String {
    let base = wallpapers_dir();
    let rel = |p: &str| {
        let p = p.replace("$HOME", &crate::home().to_string_lossy());
        Path::new(&p).strip_prefix(&base).map(|r| r.to_string_lossy().into_owned()).unwrap_or_default()
    };
    let live = std::fs::read_to_string(cfg().join("hypr/conf/livewallpaper.conf")).unwrap_or_default();
    for l in live.lines().filter(|l| !l.trim_start().starts_with('#') && l.contains("mpvpaper")) {
        if let Some(last) = l.rsplit('"').nth(1) {
            let r = rel(last);
            if !r.is_empty() {
                return r;
            }
        }
    }
    let q = run("awww", &["query"]).unwrap_or_default();
    q.lines().find_map(|l| l.split_once("image: ").map(|(_, p)| rel(p.trim()))).unwrap_or_default()
}

fn machin() -> (Option<String>, String) {
    if !crate::home().join(".local/bin/machin").exists() {
        return (None, String::new());
    }
    let dir = crate::runtime_dir().parent().map(|p| p.join("machin")).unwrap_or_default();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        let Some((pid, rol)) = n.split_once('-') else { continue };
        if !matches!(rol, "principal" | "presta") || !Path::new(&format!("/proc/{pid}")).exists() {
            continue;
        }
        let otra = std::fs::read_to_string(e.path()).unwrap_or_default();
        let otra = otra.split_whitespace().nth(1).unwrap_or("").to_string();
        return (Some(rol.to_string()), otra);
    }
    (Some(String::new()), String::new())
}

pub fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()).unwrap_or_else(|_| "compu".into())
}

/// El estado sin el tema (lo pone `tema::Tema`) ni las otras compus (mDNS).
pub fn leer() -> Estado {
    let idioma = idioma();
    let (machin, machin_otra) = machin();
    Estado {
        maquina: hostname(),
        musica: musica(),
        volumen: volumen(),
        brillo: brillo(),
        look: look_actual(),
        looks: vec![],
        wallpaper: wallpaper(),
        machin,
        machin_otra,
        idioma,
        tema: json!(null),
        otras: vec![],
    }
}
