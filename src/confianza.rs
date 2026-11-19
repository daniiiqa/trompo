//! Quién puede manejar esta compu. `trompo` (en la terminal) deja un código de
//! un solo uso que vale 5 minutos y lo enseña como QR; el cel que lo trae se
//! gana una llave propia (una cookie que solo viaja por https a esta compu).
//! Aquí se guarda solo el sha256 de cada llave, nunca la llave.

use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const VIGENCIA: u64 = 300;

#[derive(Serialize, Deserialize, Clone)]
pub struct Cel {
    pub nombre: String,
    pub hash: String,
    pub desde: u64,
}

fn file() -> PathBuf {
    crate::config_dir().join("confianza.json")
}

/// El código pendiente: lo escribe `trompo` y lo borra quien lo usa.
pub fn codigo_file() -> PathBuf {
    crate::runtime_dir().join("codigo")
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn random_hex(bytes: usize) -> String {
    let mut b = vec![0u8; bytes];
    rand::rng().fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn sha(s: &str) -> String {
    Sha256::digest(s.as_bytes()).iter().map(|x| format!("{x:02x}")).collect()
}

/// Comparación que tarda lo mismo acierte o no.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn cels() -> Vec<Cel> {
    std::fs::read_to_string(file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn guardar(list: &[Cel]) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let _ = std::fs::create_dir_all(crate::config_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(file()) {
        let _ = f.write_all(serde_json::to_string_pretty(list).unwrap_or_default().as_bytes());
    }
}

/// ¿Esta llave es de un cel emparejado? Devuelve su nombre.
pub fn quien(llave: &str) -> Option<String> {
    let h = sha(llave);
    cels().into_iter().find(|c| same(&c.hash, &h)).map(|c| c.nombre)
}

/// Un código nuevo (el de `trompo`): reemplaza al anterior.
pub fn nuevo_codigo() -> std::io::Result<String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let codigo = random_hex(16);
    let _ = std::fs::create_dir_all(crate::runtime_dir());
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(codigo_file())?;
    writeln!(f, "{codigo} {}", now() + VIGENCIA)?;
    Ok(codigo)
}

/// Canjea el código por una llave nueva para `nombre`. El código se gasta
/// aunque falle algo después: no se puede reintentar con el mismo.
pub fn canjear(codigo: &str, nombre: &str) -> Option<String> {
    let text = std::fs::read_to_string(codigo_file()).ok()?;
    let mut it = text.split_whitespace();
    let (bueno, vence) = (it.next()?, it.next()?.parse::<u64>().ok()?);
    if !same(bueno, codigo) || now() > vence {
        return None;
    }
    let _ = std::fs::remove_file(codigo_file());
    let llave = random_hex(32);
    let nombre: String = nombre.chars().filter(|c| !c.is_control()).take(40).collect();
    let nombre = if nombre.trim().is_empty() { "cel".to_string() } else { nombre.trim().to_string() };
    let mut list = cels();
    list.push(Cel { nombre, hash: sha(&llave), desde: now() });
    guardar(&list);
    Some(llave)
}

/// Olvida un cel (por nombre). Devuelve cuántos quitó.
pub fn olvidar(nombre: &str) -> usize {
    let mut list = cels();
    let antes = list.len();
    list.retain(|c| c.nombre != nombre);
    guardar(&list);
    antes - list.len()
}

/// Olvida el cel de esta llave (el que pide "olvídame").
pub fn olvidar_llave(llave: &str) {
    let h = sha(llave);
    let mut list = cels();
    list.retain(|c| !same(&c.hash, &h));
    guardar(&list);
}
