//! HTTPS sin dominio: Chrome solo instala una PWA servida por https, y en la
//! casa no hay certificado público que valga. trompo hace su propia autoridad
//! (la "CA de trompo"), que se instala una vez en el cel, y con ella firma el
//! certificado de esta compu para sus IPs de ahora.
//!
//! La CA va con *name constraints*: solo puede firmar IPs de red privada y
//! nombres `.local`. Aunque alguien se robara su llave, el cel no le creería un
//! certificado de google.com ni de tu banco.

use rcgen::{
    BasicConstraints, CertificateParams, CidrSubnet, DistinguishedName, DnType, ExtendedKeyUsagePurpose, GeneralSubtree,
    IsCa, KeyPair, KeyUsagePurpose, NameConstraints, SanType,
};
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub struct Certs {
    /// la CA en PEM, la que baja el cel
    pub ca_pem: String,
    /// el certificado de esta compu (PEM, con la CA detrás) y su llave
    pub cert_pem: String,
    pub key_pem: String,
}

fn dir() -> PathBuf {
    crate::config_dir()
}

fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    f.write_all(text.as_bytes())
}

fn private_nets() -> Vec<GeneralSubtree> {
    let net = |a: [u8; 4], prefix: u8| GeneralSubtree::IpAddress(CidrSubnet::from_v4_prefix(a, prefix));
    vec![
        net([10, 0, 0, 0], 8),
        net([172, 16, 0, 0], 12),
        net([192, 168, 0, 0], 16),
        net([100, 64, 0, 0], 10), // Tailscale y CGNAT
        GeneralSubtree::DnsName("local".into()),
    ]
}

fn ca_params(host: &str) -> CertificateParams {
    let mut p = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, format!("trompo · {host}"));
    dn.push(DnType::OrganizationName, "discada");
    p.distinguished_name = dn;
    p.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    p.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign, KeyUsagePurpose::DigitalSignature];
    p.name_constraints = Some(NameConstraints { permitted_subtrees: private_nets(), excluded_subtrees: vec![] });
    let now = SystemTime::now();
    p.not_before = (now - Duration::from_secs(86400)).into();
    p.not_after = (now + Duration::from_secs(10 * 365 * 86400)).into();
    p
}

/// Las IPs privadas de esta compu ahora mismo (las que puede ver el cel).
pub fn local_ips() -> Vec<Ipv4Addr> {
    let out = std::process::Command::new("ip").args(["-j", "-4", "addr", "show", "up"]).output();
    let v: serde_json::Value = out.ok().and_then(|o| serde_json::from_slice(&o.stdout).ok()).unwrap_or_default();
    let mut ips = Vec::new();
    for iface in v.as_array().into_iter().flatten() {
        for a in iface["addr_info"].as_array().into_iter().flatten() {
            if let Some(ip) = a["local"].as_str().and_then(|s| s.parse::<Ipv4Addr>().ok()) {
                let o = ip.octets();
                let private = ip.is_private() || (o[0] == 100 && (64..128).contains(&o[1]));
                if private && !ips.contains(&ip) {
                    ips.push(ip);
                }
            }
        }
    }
    ips
}

/// La IP con la que esta compu sale a la red (la que va en el QR).
pub fn main_ip() -> Option<Ipv4Addr> {
    let out = std::process::Command::new("ip").args(["-j", "route", "get", "1.1.1.1"]).output().ok()?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    v[0]["prefsrc"].as_str()?.parse().ok()
}

/// La CA (la hace la primera vez) y un certificado nuevo para las IPs de ahora.
pub fn load(host: &str) -> Result<Certs, String> {
    let d = dir();
    std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    let (ca_cert_path, ca_key_path) = (d.join("ca.pem"), d.join("ca.key"));

    let (ca_pem, ca_key) = match (std::fs::read_to_string(&ca_cert_path), std::fs::read_to_string(&ca_key_path)) {
        (Ok(c), Ok(k)) => (c, KeyPair::from_pem(&k).map_err(|e| format!("ca.key: {e}"))?),
        _ => {
            let key = KeyPair::generate().map_err(|e| e.to_string())?;
            let cert = ca_params(host).self_signed(&key).map_err(|e| e.to_string())?;
            write_private(&ca_key_path, &key.serialize_pem()).map_err(|e| format!("ca.key: {e}"))?;
            std::fs::write(&ca_cert_path, cert.pem()).map_err(|e| format!("ca.pem: {e}"))?;
            (cert.pem(), key)
        }
    };
    // para firmar basta con el nombre y la llave de la CA: se rearma de su PEM
    let ca = CertificateParams::from_ca_cert_pem(&ca_pem)
        .map_err(|e| format!("ca.pem: {e}"))?
        .self_signed(&ca_key)
        .map_err(|e| e.to_string())?;

    let mut p = CertificateParams::default();
    p.distinguished_name = {
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, format!("trompo · {host}"));
        dn
    };
    let mut sans = vec![SanType::DnsName(format!("{host}.local").try_into().map_err(|_| "nombre de la compu raro")?)];
    sans.extend(local_ips().into_iter().map(|ip| SanType::IpAddress(IpAddr::V4(ip))));
    p.subject_alt_names = sans;
    p.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    p.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let now = SystemTime::now();
    p.not_before = (now - Duration::from_secs(86400)).into();
    p.not_after = (now + Duration::from_secs(397 * 86400)).into();
    let key = KeyPair::generate().map_err(|e| e.to_string())?;
    let cert = p.signed_by(&key, &ca, &ca_key).map_err(|e| e.to_string())?;

    Ok(Certs { cert_pem: format!("{}{}", cert.pem(), ca_pem), key_pem: key.serialize_pem(), ca_pem })
}
