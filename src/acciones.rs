//! Lo que el cel le pide a la compu. Todo por los mismos scripts y comandos
//! que usan los atajos de discada, lanzado con `setsid -f` (como machín: un
//! hijo de un proceso que vive mucho, sin wait, se queda zombi).

use serde::Deserialize;
use std::path::Path;
use std::process::{Command, Stdio};

fn scripts() -> std::path::PathBuf {
    crate::estado::cfg().join("hypr/scripts")
}

/// PATH con ~/.local/bin (el de Hyprland no lo trae).
fn path() -> String {
    format!("{}/.local/bin:{}", crate::home().display(), std::env::var("PATH").unwrap_or_default())
}

/// Lanza y se olvida.
fn suelta(cmd: &str, args: &[&str]) {
    let _ = Command::new("setsid")
        .arg("-f")
        .arg(cmd)
        .args(args)
        .env("PATH", path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Corre y espera (cosas rápidas: wpctl, playerctl).
fn corre(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd).args(args).env("PATH", path()).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

#[derive(Deserialize)]
#[serde(tag = "que", rename_all = "kebab-case")]
pub enum Accion {
    Musica { boton: String },
    Volumen { nivel: u32 },
    Mudo,
    Brillo { nivel: u32 },
    Look { nombre: String },
    Wallpaper { archivo: String },
    Machin,
    Bloquear,
    Suspender,
}

pub fn hacer(a: Accion) -> Result<(), String> {
    match a {
        Accion::Musica { boton } => {
            let b = match boton.as_str() {
                "play-pause" | "next" | "previous" => boton.as_str(),
                _ => return Err("botón raro".into()),
            };
            corre("playerctl", &[b]).then_some(()).ok_or_else(|| "no hay nada sonando".into())
        }
        Accion::Volumen { nivel } => {
            let v = format!("{:.2}", nivel.min(100) as f64 / 100.0);
            corre("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", &v]);
            corre("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "0"]);
            Ok(())
        }
        Accion::Mudo => {
            corre("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]);
            Ok(())
        }
        Accion::Brillo { nivel } => {
            let n = format!("{}%", nivel.clamp(1, 100));
            corre("brightnessctl", &["-q", "-c", "backlight", "set", &n]);
            Ok(())
        }
        Accion::Look { nombre } => {
            let ok = crate::estado::looks("en").iter().any(|l| l.nombre == nombre);
            if !ok {
                return Err("ese look no existe".into());
            }
            suelta(&scripts().join("LookSwitch.sh").to_string_lossy(), &[&nombre]);
            Ok(())
        }
        Accion::Wallpaper { archivo } => {
            let f = wallpaper_seguro(&archivo).ok_or("ese wallpaper no está")?;
            suelta(&scripts().join("WallpaperSelect.sh").to_string_lossy(), &[&f.to_string_lossy()]);
            Ok(())
        }
        Accion::Machin => {
            suelta(&crate::home().join(".local/bin/machin").to_string_lossy(), &[]);
            Ok(())
        }
        Accion::Bloquear => {
            suelta(&scripts().join("LockScreen.sh").to_string_lossy(), &[]);
            Ok(())
        }
        Accion::Suspender => {
            suelta("systemctl", &["suspend"]);
            Ok(())
        }
    }
}

pub const IMAGENES: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp"];
pub const VIDEOS: &[&str] = &["mp4", "mkv", "mov", "webm"];

fn ext(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

pub fn es_video(p: &Path) -> bool {
    VIDEOS.contains(&ext(p).as_str())
}

/// Una ruta relativa que mandó el cel → el archivo de verdad, solo si está
/// dentro de la carpeta de wallpapers (nada de `../../.ssh`).
pub fn wallpaper_seguro(rel: &str) -> Option<std::path::PathBuf> {
    let base = crate::estado::wallpapers_dir().canonicalize().ok()?;
    let f = base.join(rel).canonicalize().ok()?;
    let e = ext(&f);
    (f.starts_with(&base) && f.is_file() && (IMAGENES.contains(&e.as_str()) || VIDEOS.contains(&e.as_str()))).then_some(f)
}

/// Los wallpapers (rutas relativas), los más nuevos primero.
pub fn wallpapers() -> Vec<String> {
    let base = crate::estado::wallpapers_dir();
    let mut out = Vec::new();
    let mut pila = vec![base.clone()];
    while let Some(d) = pila.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                pila.push(p);
                continue;
            }
            let x = ext(&p);
            if IMAGENES.contains(&x.as_str()) || VIDEOS.contains(&x.as_str()) {
                let t = e.metadata().and_then(|m| m.modified()).ok();
                if let Ok(r) = p.strip_prefix(&base) {
                    out.push((t, r.to_string_lossy().into_owned()));
                }
            }
        }
    }
    out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    out.into_iter().map(|(_, r)| r).collect()
}

/// La miniatura (jpg cuadrado de 360, o 960x540 si `ancha`) del wallpaper, hecha una vez y guardada
/// en ~/.cache/trompo/min.
pub fn miniatura(f: &Path, ancha: bool) -> Option<std::path::PathBuf> {
    let (tam, sufijo) = if ancha { ("960x540", "-a") } else { ("360x360", "") };
    let meta = std::fs::metadata(f).ok()?;
    let stamp = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    let key: u64 = f.to_string_lossy().bytes().fold(1469598103934665603u64, |h, b| (h ^ b as u64).wrapping_mul(1099511628211));
    let dir = crate::home().join(".cache/trompo/min");
    let _ = std::fs::create_dir_all(&dir);
    let out = dir.join(format!("{key:016x}-{stamp}{sufijo}.jpg"));
    if out.exists() {
        return Some(out);
    }
    let src = if es_video(f) {
        // un cuadro de en medio del video (si WallpaperSelect ya lo sacó, ese)
        let cached = crate::home().join(".cache/video_frame").join(format!("{}.png", f.file_name()?.to_string_lossy()));
        if cached.exists() {
            cached
        } else {
            let tmp = dir.join(format!("{key:016x}.png"));
            Command::new("ffmpeg")
                .args(["-y", "-loglevel", "error", "-ss", "2", "-i"])
                .arg(f)
                .args(["-frames:v", "1"])
                .arg(&tmp)
                .status()
                .ok()?;
            tmp
        }
    } else {
        f.to_path_buf()
    };
    let ok = Command::new("magick")
        .arg(format!("{}[0]", src.display()))
        .args(["-thumbnail", &format!("{tam}^"), "-gravity", "center", "-extent", tam, "-quality", "82"])
        .arg(&out)
        .status()
        .is_ok_and(|s| s.success());
    ok.then_some(out)
}
