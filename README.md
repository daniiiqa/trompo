# trompo

Your computer from your phone, in the look of the
[discada](https://github.com/daniiiqa/dotfiles) rice. A PWA: no app store, no APK —
the computer serves it and the phone installs it from Chrome. *Lo pones a bailar
desde el cel.*

```
trompo            pair a phone: a one-time QR in the terminal (5 minutes)
trompo sirve      the daemon (Hyprland's startup.lua)
trompo cels       paired phones
trompo olvida N   forget phone N
```

- **Now playing** (any MPRIS player, through `playerctl`): cover, title, progress,
  play/pause/next/previous.
- **Volume** (`wpctl`) and, on laptops, **brightness** (`brightnessctl`).
- **Looks**: the list from `lena`, with each fixed palette as a swatch; tap one and
  `LookSwitch.sh` puts it on. The phone repaints itself with the new look through
  an iris from where you tapped, like farula.
- **Wallpapers**: thumbnails of `~/Pictures/wallpapers` (images and videos, cached in
  `~/.cache/trompo`); tap one and `WallpaperSelect.sh <path>` sets it.
- **machín** on/off, **lock** and **sleep** (those two want a second tap).
- **Your other computers** running trompo show up on top (mDNS `_trompo._tcp`).

The whole page follows the computer's look live — the same colors and `[style]` as
rola, vato and taco (via [disco](https://github.com/daniiiqa/disco)): borders,
corners, bars, case, how the selection is marked, and the look's own font, which
the computer serves to the phone.

## The phone, once

1. On the computer: `trompo`. Scan the QR with the phone's camera (same Wi-Fi).
2. The first time, the page asks you to install **trompo's certificate** (Chrome only
   installs PWAs served over https): download it, then *Settings → Security and
   privacy → More security settings → Install from device storage → CA certificate*.
   The page continues on its own once the phone trusts it.
3. Name the phone, tap *Emparejar*, then Chrome's menu → *Install app*.

Give the computer a fixed IP in your router (DHCP reservation): the installed app
lives at `https://<ip>:47421`.

## Security

- trompo makes its own certificate authority in `~/.config/trompo/` (`ca.key` is
  0600). It carries **name constraints**: it can only vouch for private IPs
  (10/8, 172.16/12, 192.168/16, 100.64/10) and `.local` names — even with its key,
  nobody can mint a certificate the phone would accept for a real website.
- Pairing codes are 128-bit, single use, 5 minutes. A paired phone gets its own
  256-bit key as an `HttpOnly; Secure; SameSite=Strict` cookie; the computer keeps
  only its sha256 (`~/.config/trompo/confianza.json`).
- Every `/api` call needs that cookie and a same-origin `Origin`. Wallpaper paths
  are canonicalized and must stay inside the wallpapers folder; cover art is only
  served if it is really an image.
- Port 47420 (plain http) only serves the welcome page and the CA certificate.

## Install

```sh
cargo install --path . --root ~/.local
```

Needs: `playerctl`, `wireplumber` (`wpctl`), `imagemagick` (thumbnails), `ffmpeg`
(video thumbnails), `fontconfig`; optional `brightnessctl`, `lena`, `machin`.
Run `trompo sirve` at login (discada does it in `startup.lua`).
