/// Mappar win-rate (0..1) till röd → gul → grön hex-färg.
pub fn win_color(p: f64) -> String {
    let p = p.clamp(0.0, 1.0);
    let h = p * 120.0;
    let s = 0.65;
    let l = 0.55 - (p - 0.5).abs() * 0.10;
    hsl_to_hex(h, s, l)
}

/// Mappar en z-score i [-clamp, +clamp] till röd → gul → grön.
pub fn z_color(z: f64, clamp: f64) -> String {
    let p = ((z / clamp) + 1.0) / 2.0;
    win_color(p.clamp(0.0, 1.0))
}

fn hsl_to_hex(h: f64, s: f64, l: f64) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to8 = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", to8(r), to8(g), to8(b))
}
