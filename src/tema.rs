//! El look de discada para la página: los mismos colores y estilo que rola,
//! vato y taco (disco::look), más la fuente del look (el cel no la tiene: se
//! la sirve la compu).

use disco::look::{Bar, Borders, Case, Look, Select};
use ratatui::style::Color;
use ratatui::symbols::border;
use serde_json::{Value, json};
use std::path::PathBuf;

pub struct Tema {
    look: Look,
    pub fuente: Option<PathBuf>,
    familia: String,
}

fn hex(c: Color, fallback: &str) -> String {
    match c {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        _ => fallback.to_string(),
    }
}

/// El archivo de la fuente del look (fc-match), regular.
/// kitty acepta el estilo pegado al nombre ("GeistMono Nerd Font Mono Bold"):
/// si fc-match no reconoce la familia (devuelve otra), se le quita la última
/// palabra y se vuelve a probar. En el cel va la regular.
fn font_file(family: &str) -> Option<PathBuf> {
    let mut name = family.trim().to_string();
    for _ in 0..3 {
        let o = std::process::Command::new("fc-match").args(["-f", "%{family}\x1f%{file}", &format!("{name}:style=Regular")]).output().ok()?;
        let out = String::from_utf8_lossy(&o.stdout).into_owned();
        let (fams, file) = out.split_once('\x1f')?;
        if fams.split(',').any(|f| f.eq_ignore_ascii_case(&name)) {
            let p = PathBuf::from(file.trim());
            return p.exists().then_some(p);
        }
        name = name.rsplit_once(' ')?.0.to_string();
    }
    None
}

impl Tema {
    pub fn new() -> Tema {
        let mut t = Tema { look: Look::load("trompo"), fuente: None, familia: String::new() };
        t.fuente_del_look();
        t
    }

    fn fuente_del_look(&mut self) {
        self.familia = disco::look::kitty("font_family").unwrap_or_default();
        // "family=GeistMono Nerd Font Mono style=…" (kitty 0.36+) o el nombre a secas
        if let Some(rest) = self.familia.strip_prefix("family=") {
            self.familia = rest.split(" style=").next().unwrap_or(rest).trim_matches('"').to_string();
        }
        self.fuente = if self.familia.is_empty() { None } else { font_file(&self.familia) };
    }

    /// ¿Cambió el look? (otro theme de kitty, otro estilo)
    pub fn refresh(&mut self) -> bool {
        let changed = self.look.refresh();
        if changed {
            self.fuente_del_look();
        }
        changed
    }

    pub fn json(&self) -> Value {
        let l = &self.look;
        let t = &l.theme;
        let base = hex(t.base, "#101014");
        let set = l.border_set();
        let rounded = set == border::ROUNDED;
        let (bw, bstyle) = match l.borders {
            Borders::None => (0, "none"),
            Borders::Double => (3, "double"),
            Borders::Thick => (2, "solid"),
            Borders::Dashed => (1, "dashed"),
            _ => (1, "solid"),
        };
        let fuente_v = self
            .fuente
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok()?.modified().ok())
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let fmt = match self.fuente.as_ref().and_then(|p| p.extension()).and_then(|e| e.to_str()) {
            Some("otf") => "opentype",
            Some("woff2") => "woff2",
            _ => "truetype",
        };
        json!({
            "base": base,
            "primary": hex(t.primary, "#7aa2f7"),
            "accent": hex(t.accent, "#e0af68"),
            "fg": hex(t.fg, "#e6e6e6"),
            "muted": hex(t.muted, "#808080"),
            "border": hex(t.border, "#404040"),
            "hl": hex(t.hl, "#303040"),
            "red": hex(t.red, "#e0452b"),
            "green": hex(t.green, "#8fa86a"),
            "radius": if rounded { 14 } else { 0 },
            "borderWidth": bw,
            "borderStyle": bstyle,
            "case": match l.case { Case::Upper => "uppercase", Case::Lower => "lowercase", Case::Normal => "none" },
            "select": match l.select { Select::Fill => "fill", Select::Bar => "bar", Select::Inverse => "inverse" },
            "bar": match l.bar { Bar::Line => "line", Bar::Knob => "knob", Bar::Blocks => "blocks", Bar::Dots => "dots", Bar::Ascii => "ascii" },
            "icons": l.icons,
            "transparent": l.transparent,
            "familia": self.familia,
            "fuente": self.fuente.as_ref().map(|_| format!("/api/fuente?v={fuente_v}")),
            "fuenteFormato": fmt,
        })
    }
}
