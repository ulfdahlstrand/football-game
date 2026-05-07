use std::path::Path;

use super::helpers::{svg_header, write_svg_file};

pub struct SessionProgress {
    pub session: String,
    pub goal_diff: f64,
    pub improved: bool,
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

    let mut out = svg_header(width, height);
    out.push_str("  <text x=\"72\" y=\"34\" font-family=\"Arial, sans-serif\" font-size=\"20\" font-weight=\"700\" fill=\"#111827\">Improvement per session</text>\n");
    let improved_count = sessions.iter().filter(|s| s.improved).count();
    out.push_str(&format!(
        "  <text x=\"{}\" y=\"34\" text-anchor=\"end\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#6b7280\">{}/{} sessions improved baseline</text>\n",
        (width - pad_right) as i64, improved_count, n
    ));

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

    out.push_str(&format!(
        "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#374151\" stroke-width=\"1.4\"/>\n",
        pad_left, pad_top, pad_left, pad_top + plot_h
    ));
    out.push_str(&format!(
        "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#374151\" stroke-width=\"1.4\"/>\n",
        pad_left, pad_top + plot_h, width - pad_right, pad_top + plot_h
    ));

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

        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#374151\">{}</text>\n",
            cx, pad_top + plot_h + 20.0, s.session
        ));
    }

    let mid_y = pad_top + plot_h / 2.0;
    out.push_str(&format!(
        "  <text x=\"14\" y=\"{:.1}\" transform=\"rotate(-90 14 {:.1})\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#374151\">Goal diff vs prev baseline</text>\n",
        mid_y, mid_y
    ));

    out.push_str("</svg>\n");
    write_svg_file(path, &out);
}
