//! trompo: tu compu desde el cel. Una PWA con el look de discada para la
//! música, el volumen, los looks, los wallpapers y machín.
//!
//!   trompo            empareja un cel: QR en la terminal (vale 5 minutos)
//!   trompo sirve      el daemon (startup.lua)
//!   trompo cels       los cels emparejados
//!   trompo olvida N   olvida el cel N

mod acciones;
mod confianza;
mod estado;
mod sirve;
mod tema;
mod tls;
mod web;

use std::path::PathBuf;
use std::process::ExitCode;

pub fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
}

pub fn config_dir() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| home().join(".config")).join("trompo")
}

pub fn runtime_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("trompo")
}

fn es() -> bool {
    estado::idioma() == "es"
}

fn l<'a>(es_: &'a str, en: &'a str) -> &'a str {
    if es() { es_ } else { en }
}

/// El QR con medios bloques: dos módulos por renglón, con su margen.
fn qr(texto: &str) -> String {
    let code = match qrcode::QrCode::with_error_correction_level(texto, qrcode::EcLevel::L) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let w = code.width();
    let colors = code.to_colors();
    let dark = |x: isize, y: isize| x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < w && colors[y as usize * w + x as usize] == qrcode::Color::Dark;
    let q = 2isize;
    let mut out = String::new();
    let mut y = -q;
    while y < w as isize + q {
        out.push_str("  \x1b[97;40m");
        for x in -q..w as isize + q {
            // fondo claro, módulos oscuros (los lectores lo prefieren así)
            out.push(match (dark(x, y), dark(x, y + 1)) {
                (false, false) => '█',
                (true, false) => '▄',
                (false, true) => '▀',
                (true, true) => ' ',
            });
        }
        out.push_str("\x1b[0m\n");
        y += 2;
    }
    out
}

fn sirviendo() -> bool {
    std::fs::read_to_string(runtime_dir().join("sirve.pid"))
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .is_some_and(|pid| std::path::Path::new(&format!("/proc/{pid}")).exists())
}

fn emparejar() -> ExitCode {
    if !sirviendo() {
        eprintln!("{}", l("trompo no está sirviendo: arráncalo con `trompo sirve` (o reinicia la sesión)", "trompo isn't serving: start it with `trompo sirve` (or log in again)"));
        return ExitCode::FAILURE;
    }
    let Some(ip) = tls::main_ip() else {
        eprintln!("{}", l("sin red: conecta la compu al Wi-Fi del cel", "no network: put the computer on the phone's Wi-Fi"));
        return ExitCode::FAILURE;
    };
    let codigo = match confianza::nuevo_codigo() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("trompo: {e}");
            return ExitCode::FAILURE;
        }
    };
    let url = format!("http://{ip}:{}/#c={codigo}", sirve::HTTP);
    println!();
    print!("{}", qr(&url));
    println!();
    println!("  \x1b[1mtrompo\x1b[0m · {}", estado::hostname());
    println!("  {}", l("Escanéalo con la cámara del cel (mismo Wi-Fi).", "Scan it with the phone's camera (same Wi-Fi)."));
    println!("  {}", l("La primera vez te pide instalar el certificado de trompo.", "The first time it asks you to install trompo's certificate."));
    println!("  \x1b[2m{url}\x1b[0m");
    println!("  \x1b[2m{}\x1b[0m", l("vale 5 minutos y una sola vez · Ctrl+C para cancelar", "valid for 5 minutes, once · Ctrl+C to cancel"));

    // esperar a que el cel lo canjee (el daemon borra el archivo) o se venza
    let antes = confianza::cels().len();
    let hasta = confianza::now() + confianza::VIGENCIA;
    while confianza::now() < hasta {
        std::thread::sleep(std::time::Duration::from_millis(400));
        if !confianza::codigo_file().exists() {
            let cels = confianza::cels();
            if cels.len() > antes {
                let n = cels.last().map(|c| c.nombre.clone()).unwrap_or_default();
                println!("\n  \x1b[32m✓\x1b[0m {} {n}", l("listo:", "done:"));
                return ExitCode::SUCCESS;
            }
            println!("\n  {}", l("otro `trompo` reemplazó este código", "another `trompo` replaced this code"));
            return ExitCode::FAILURE;
        }
    }
    let _ = std::fs::remove_file(confianza::codigo_file());
    println!("\n  {}", l("se venció el código; corre `trompo` otra vez", "the code expired; run `trompo` again"));
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("empareja") => emparejar(),
        Some("sirve") => {
            let rt = tokio::runtime::Runtime::new().expect("tokio");
            match rt.block_on(sirve::sirve()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("trompo: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("cels") => {
            let cels = confianza::cels();
            if cels.is_empty() {
                println!("{}", l("ningún cel emparejado (`trompo` para emparejar uno)", "no phones paired (`trompo` to pair one)"));
            }
            for c in cels {
                println!("{}", c.nombre);
            }
            ExitCode::SUCCESS
        }
        Some("olvida") => match args.get(1) {
            Some(n) => {
                let k = confianza::olvidar(n);
                println!("{}", if k > 0 { l("olvidado", "forgotten") } else { l("no había ninguno con ese nombre", "no phone with that name") });
                ExitCode::SUCCESS
            }
            None => {
                eprintln!("trompo olvida <nombre>   ({})", l("ver `trompo cels`", "see `trompo cels`"));
                ExitCode::FAILURE
            }
        },
        Some("-V" | "--version") => {
            println!("trompo {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            println!("trompo {}\n", env!("CARGO_PKG_VERSION"));
            println!("  trompo            {}", l("empareja un cel (QR)", "pair a phone (QR)"));
            println!("  trompo sirve      {}", l("el daemon (startup.lua)", "the daemon (startup.lua)"));
            println!("  trompo cels       {}", l("los cels emparejados", "paired phones"));
            println!("  trompo olvida N   {}", l("olvida el cel N", "forget phone N"));
            ExitCode::SUCCESS
        }
    }
}
