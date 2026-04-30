use std::path::Path;
use std::fs;

use crate::session::EpochSummary;

pub struct SessionProgress {
    pub session: String,
    pub goal_diff: f64,
    pub improved: bool,
}

#[derive(Clone, Default)]
pub struct MatrixCell {
    pub team0_wins: u32,
    pub team1_wins: u32,
    pub draws: u32,
    pub team0_goals: u64,
    pub team1_goals: u64,
    pub games: u32,
}

/// Maps win-rate (0..1) to a red→yellow→green hex color.
fn win_color(p: f64) -> String {
    let p = p.clamp(0.0, 1.0);
    // hue: 0 (red) at p=0, 60 (yellow) at p=0.5, 120 (green) at p=1.0
    let h = p * 120.0;
    let s = 0.65;
    let l = 0.55 - (p - 0.5).abs() * 0.10; // slightly darker at extremes
    hsl_to_hex(h, s, l)
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

pub fn write_matrix_svg(
    path: &Path,
    names: &[&str],
    matrix: &[Vec<MatrixCell>],
    total_matches: u64,
    training_minutes: f64,
) {
    let n = names.len();
    if n == 0 { return; }

    let cell_size: f64 = 64.0;
    let label_w: f64 = 80.0;
    let label_h: f64 = 60.0;
    let pad_top: f64 = 70.0;
    let pad_bottom: f64 = 70.0;
    let pad_left: f64 = 30.0;

    let grid_w = cell_size * n as f64;
    let grid_h = cell_size * n as f64;
    let width = pad_left + label_w + grid_w + 30.0;
    let height = pad_top + label_h + grid_h + pad_bottom;

    let grid_x = pad_left + label_w;
    let grid_y = pad_top + label_h;

    let mut out = String::with_capacity(16 * 1024);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        width as i64, height as i64, width as i64, height as i64
    ));
    out.push_str("  <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");

    // Title
    out.push_str(&format!(
        "  <text x=\"{}\" y=\"34\" font-family=\"Arial, sans-serif\" font-size=\"22\" font-weight=\"700\" fill=\"#111827\">Round-robin: row team home win %</text>\n",
        pad_left as i64
    ));
    out.push_str(&format!(
        "  <text x=\"{}\" y=\"56\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#6b7280\">Each cell: row plays as team 0 (home) vs column as team 1 (away). Color: 0% red — 50% yellow — 100% green.</text>\n",
        pad_left as i64
    ));

    // Column labels (top, rotated)
    for (j, name) in names.iter().enumerate() {
        let x = grid_x + cell_size * j as f64 + cell_size / 2.0;
        let y = grid_y - 8.0;
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" transform=\"rotate(-45 {:.1} {:.1})\" text-anchor=\"start\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#374151\">{}</text>\n",
            x, y, x, y, name
        ));
    }

    // Row labels (left)
    for (i, name) in names.iter().enumerate() {
        let x = grid_x - 8.0;
        let y = grid_y + cell_size * i as f64 + cell_size / 2.0 + 4.0;
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#374151\">{}</text>\n",
            x, y, name
        ));
    }

    // Cells
    for (i, row) in matrix.iter().enumerate() {
        for (j, cell) in row.iter().enumerate() {
            let win_pct = if cell.games > 0 {
                (cell.team0_wins as f64 + cell.draws as f64 * 0.5) / cell.games as f64
            } else { 0.5 };

            let cx = grid_x + cell_size * j as f64;
            let cy = grid_y + cell_size * i as f64;
            let color = win_color(win_pct);

            out.push_str(&format!(
                "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\" stroke=\"#ffffff\" stroke-width=\"1.5\"/>\n",
                cx, cy, cell_size, cell_size, color
            ));

            // Diagonal: lighter overlay so self-vs-self stands out a bit
            if i == j {
                out.push_str(&format!(
                    "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#1f2937\" stroke-width=\"1.5\" stroke-dasharray=\"3 2\"/>\n",
                    cx + 2.0, cy + 2.0, cell_size - 4.0, cell_size - 4.0
                ));
            }

            // Win % overlay
            let pct_text = format!("{:.0}%", win_pct * 100.0);
            let text_color = if (win_pct - 0.5).abs() > 0.35 { "#ffffff" } else { "#111827" };
            out.push_str(&format!(
                "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"14\" font-weight=\"700\" fill=\"{}\">{}</text>\n",
                cx + cell_size / 2.0, cy + cell_size / 2.0 + 5.0, text_color, pct_text
            ));
        }
    }

    // Footer with totals
    let footer_y = grid_y + grid_h + 36.0;
    out.push_str(&format!(
        "  <text x=\"{}\" y=\"{:.1}\" font-family=\"Arial, sans-serif\" font-size=\"13\" fill=\"#111827\"><tspan font-weight=\"700\">Total matches played:</tspan> {}</text>\n",
        pad_left as i64, footer_y, format_thousands(total_matches)
    ));
    out.push_str(&format!(
        "  <text x=\"{}\" y=\"{:.1}\" font-family=\"Arial, sans-serif\" font-size=\"13\" fill=\"#111827\"><tspan font-weight=\"700\">Total training time:</tspan> {:.1} minutes</text>\n",
        pad_left as i64, footer_y + 22.0, training_minutes
    ));

    out.push_str("</svg>\n");

    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
    let _ = fs::write(path, out);
}

fn format_thousands(n: u64) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().rev().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && i % 3 == 0 { out.push(' '); }
        out.push(*c);
    }
    out.chars().rev().collect()
}

pub fn write_progress_svg(path: &Path, sessions: &[SessionProgress]) {
    if sessions.is_empty() { return; }

    let width: f64 = 900.0;
    let height: f64 = 480.0;
    let pad_left: f64 = 72.0;
    let pad_right: f64 = 32.0;
    let pad_top: f64 = 54.0;
    let pad_bottom: f64 = 72.0;
    let plot_w = width - pad_left - pad_right;
    let plot_h = height - pad_top - pad_bottom;

    let n = sessions.len();
    let max_diff = sessions.iter().map(|s| s.goal_diff).fold(0.0_f64, f64::max);
    let y_max = (max_diff * 1.15).max(100.0);

    let bar_w = (plot_w / n as f64 * 0.6).min(80.0);
    let gap = plot_w / n as f64;

    let x_center = |i: usize| pad_left + gap * i as f64 + gap / 2.0;
    let bar_h = |diff: f64| (diff / y_max * plot_h).max(2.0);
    let bar_y = |diff: f64| pad_top + plot_h - bar_h(diff);

    let mut out = String::with_capacity(4096);

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        width as i64, height as i64, width as i64, height as i64
    ));
    out.push_str("  <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");
    out.push_str("  <text x=\"72\" y=\"34\" font-family=\"Arial, sans-serif\" font-size=\"20\" font-weight=\"700\" fill=\"#111827\">Improvement per session</text>\n");
    let improved_count = sessions.iter().filter(|s| s.improved).count();
    out.push_str(&format!(
        "  <text x=\"{}\" y=\"34\" text-anchor=\"end\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#6b7280\">{}/{} sessions improved baseline</text>\n",
        (width - pad_right) as i64, improved_count, n
    ));

    // Y grid lines
    let steps = 5;
    for i in 0..=steps {
        let value = y_max * i as f64 / steps as f64;
        let y = pad_top + plot_h - (value / y_max * plot_h);
        out.push_str(&format!(
            "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#e5e7eb\" stroke-width=\"1\"/>\n",
            pad_left, y, width - pad_right, y
        ));
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#6b7280\">{:.0}</text>\n",
            pad_left - 8.0, y + 4.0, value
        ));
    }

    // Axes
    out.push_str(&format!(
        "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#374151\" stroke-width=\"1.4\"/>\n",
        pad_left, pad_top, pad_left, pad_top + plot_h
    ));
    out.push_str(&format!(
        "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#374151\" stroke-width=\"1.4\"/>\n",
        pad_left, pad_top + plot_h, width - pad_right, pad_top + plot_h
    ));

    // Bars + labels
    for (i, s) in sessions.iter().enumerate() {
        let cx = x_center(i);
        let bx = cx - bar_w / 2.0;

        if s.improved {
            let bh = bar_h(s.goal_diff);
            let by = bar_y(s.goal_diff);
            let intensity = (s.goal_diff / y_max).min(1.0);
            let g_val = (180.0 - intensity * 80.0) as u8;
            let color = format!("#16{:02x}4a", g_val);

            out.push_str(&format!(
                "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\" rx=\"3\"/>\n",
                bx, by, bar_w, bh, color
            ));
            out.push_str(&format!(
                "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"12\" font-weight=\"600\" fill=\"#111827\">{:+.0}</text>\n",
                cx, by - 6.0, s.goal_diff
            ));
        } else {
            // Did not improve baseline: small gray marker at axis
            let by = pad_top + plot_h - 4.0;
            out.push_str(&format!(
                "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"4\" fill=\"#d1d5db\" rx=\"2\"/>\n",
                bx, by, bar_w
            ));
            out.push_str(&format!(
                "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#9ca3af\">—</text>\n",
                cx, by - 6.0
            ));
        }

        // Session name below axis
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#374151\">{}</text>\n",
            cx, pad_top + plot_h + 20.0, s.session
        ));
    }

    // Y axis label
    let mid_y = pad_top + plot_h / 2.0;
    out.push_str(&format!(
        "  <text x=\"14\" y=\"{:.1}\" transform=\"rotate(-90 14 {:.1})\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#374151\">Goal diff vs prev baseline</text>\n",
        mid_y, mid_y
    ));

    out.push_str("</svg>\n");

    let _ = fs::write(path, out);
}

pub fn write_training_svg(path: &Path, history: &[EpochSummary], final_champion_epoch: usize) {
    let width: f64 = 1100.0;
    let height: f64 = 620.0;
    let pad_left: f64 = 72.0;
    let pad_right: f64 = 32.0;
    let pad_top: f64 = 54.0;
    let pad_bottom: f64 = 78.0;
    let plot_w = width - pad_left - pad_right;
    let plot_h = height - pad_top - pad_bottom;
    let epochs = history.len().max(1) as f64;

    let goal_values: Vec<f64> = history.iter().flat_map(|h| {
        [h.baseline_avg_goals, h.candidate_avg_goals, h.goal_diff]
    }).collect();

    let min_y = goal_values.iter().cloned().fold(-1.0_f64, f64::min);
    let max_y = goal_values.iter().cloned().fold(3.0_f64, f64::max);
    let y_span = (max_y - min_y).max(1.0);

    let x_for = |i: usize| -> f64 {
        pad_left + (i as f64 / (epochs - 1.0).max(1.0)) * plot_w
    };
    let y_for = |value: f64| -> f64 {
        pad_top + (1.0 - (value - min_y) / y_span) * plot_h
    };

    let polyline_pts = |key: fn(&EpochSummary) -> f64| -> String {
        history.iter().enumerate()
            .map(|(i, h)| format!("{:.1},{:.1}", x_for(i), y_for(key(h))))
            .collect::<Vec<_>>().join(" ")
    };

    let mut out = String::with_capacity(8192);

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        width as i64, height as i64, width as i64, height as i64
    ));
    out.push_str("  <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");

    // Title
    out.push_str(&format!(
        "  <text x=\"{}\" y=\"30\" font-family=\"Arial, sans-serif\" font-size=\"22\" font-weight=\"700\" fill=\"#111827\">Training progress</text>\n",
        pad_left as i64
    ));
    out.push_str(&format!(
        "  <text x=\"{}\" y=\"30\" text-anchor=\"end\" font-family=\"Arial, sans-serif\" font-size=\"13\" fill=\"#4b5563\">Final champion epoch: {}</text>\n",
        (width - pad_right) as i64, final_champion_epoch
    ));
    out.push_str("  <g font-family=\"Arial, sans-serif\">\n");

    // X-axis ticks
    let tick_step = ((epochs / 10.0).round() as usize).max(1);
    let mut e = 0usize;
    while e < history.len() {
        let x = x_for(e);
        out.push_str(&format!(
            "    <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#eef2f7\" stroke-width=\"1\"/>\n",
            x, pad_top, x, height - pad_bottom
        ));
        out.push_str(&format!(
            "    <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#4b5563\">{}</text>\n",
            x, height - pad_bottom + 24.0, e + 1
        ));
        e += tick_step;
    }

    // Y-axis grid
    for i in 0..=5 {
        let value = min_y + y_span * i as f64 / 5.0;
        let y = y_for(value);
        out.push_str(&format!(
            "    <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#d8dee9\" stroke-width=\"1\" opacity=\"0.55\"/>\n",
            pad_left, y, width - pad_right, y
        ));
        out.push_str(&format!(
            "    <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-size=\"12\" fill=\"#4b5563\">{:.1}</text>\n",
            pad_left - 12.0, y + 4.0, value
        ));
    }

    // Axes
    out.push_str(&format!(
        "    <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#111827\" stroke-width=\"1.4\"/>\n",
        pad_left, pad_top, pad_left, height - pad_bottom
    ));
    out.push_str(&format!(
        "    <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#111827\" stroke-width=\"1.4\"/>\n",
        pad_left, height - pad_bottom, width - pad_right, height - pad_bottom
    ));

    // Axis labels
    out.push_str(&format!(
        "    <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"13\" fill=\"#111827\">Epoch</text>\n",
        pad_left + plot_w / 2.0, height - 24.0
    ));
    let mid_y = pad_top + plot_h / 2.0;
    out.push_str(&format!(
        "    <text x=\"20\" y=\"{:.1}\" transform=\"rotate(-90 20 {:.1})\" text-anchor=\"middle\" font-size=\"13\" fill=\"#111827\">Goals / goal diff</text>\n",
        mid_y, mid_y
    ));

    // Lines
    out.push_str(&format!(
        "    <polyline fill=\"none\" stroke=\"#2563eb\" stroke-width=\"2.4\" points=\"{}\"/>\n",
        polyline_pts(|h| h.candidate_avg_goals)
    ));
    out.push_str(&format!(
        "    <polyline fill=\"none\" stroke=\"#ef4444\" stroke-width=\"2.4\" points=\"{}\"/>\n",
        polyline_pts(|h| h.baseline_avg_goals)
    ));
    out.push_str(&format!(
        "    <polyline fill=\"none\" stroke=\"#7c3aed\" stroke-width=\"2\" stroke-dasharray=\"6 5\" points=\"{}\"/>\n",
        polyline_pts(|h| h.goal_diff)
    ));

    // Accepted dots
    for (i, h) in history.iter().enumerate() {
        if h.accepted {
            let x = x_for(i);
            let y = y_for(h.goal_diff);
            out.push_str(&format!(
                "    <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"5\" fill=\"#16a34a\"><title>Accepted epoch {}, diff {}</title></circle>\n",
                x, y, h.epoch, h.goal_diff
            ));
        }
    }

    // Legend
    let legend_y = height - 54.0;
    out.push_str(&format!("    <g transform=\"translate({}, {:.1})\" font-size=\"13\" fill=\"#111827\">\n", pad_left as i64, legend_y));
    out.push_str("      <rect x=\"0\" y=\"-12\" width=\"14\" height=\"4\" fill=\"#2563eb\"/><text x=\"22\" y=\"-7\">Candidate avg goals</text>\n");
    out.push_str("      <rect x=\"190\" y=\"-12\" width=\"14\" height=\"4\" fill=\"#ef4444\"/><text x=\"212\" y=\"-7\">Baseline avg goals</text>\n");
    out.push_str("      <rect x=\"378\" y=\"-12\" width=\"14\" height=\"4\" fill=\"#7c3aed\"/><text x=\"400\" y=\"-7\">Goal diff</text>\n");
    out.push_str("      <circle cx=\"548\" cy=\"-10\" r=\"5\" fill=\"#16a34a\"/><text x=\"562\" y=\"-7\">Accepted improvement</text>\n");
    out.push_str("    </g>\n");

    out.push_str("  </g>\n</svg>\n");

    let _ = fs::write(path, out);
}
