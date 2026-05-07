use std::fs;
use std::path::Path;

/// XML-deklaration + `<svg>`-öppningstag + vit bakgrundsruta.
pub fn svg_header(width: f64, height: f64) -> String {
    let mut s = String::with_capacity(256);
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        width as i64, height as i64, width as i64, height as i64
    ));
    s.push_str("  <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");
    s
}

/// Skapar parent-katalog vid behov och skriver innehållet.
pub fn write_svg_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
    let _ = fs::write(path, content);
}

/// Formaterar ett heltal med mellanslag som tusen-separator (12345 → "12 345").
pub fn format_thousands(n: u64) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().rev().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && i % 3 == 0 { out.push(' '); }
        out.push(*c);
    }
    out.chars().rev().collect()
}
