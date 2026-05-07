use std::path::Path;

use super::colors::win_color;
use super::helpers::{format_thousands, svg_header, write_svg_file};

#[derive(Clone, Default)]
pub struct MatrixCell {
    pub team0_wins: u32,
    pub team1_wins: u32,
    pub draws: u32,
    pub team0_goals: u64,
    pub team1_goals: u64,
    pub games: u32,
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

    let mut out = svg_header(width, height);

    out.push_str(&format!(
        "  <text x=\"{}\" y=\"34\" font-family=\"Arial, sans-serif\" font-size=\"22\" font-weight=\"700\" fill=\"#111827\">Round-robin (side-swapped): row vs column win %</text>\n",
        pad_left as i64
    ));
    out.push_str(&format!(
        "  <text x=\"{}\" y=\"56\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#6b7280\">Each pair plays half home, half away to remove positional bias. Diagonal ≈ 50%. Color: 0% red — 50% yellow — 100% green.</text>\n",
        pad_left as i64
    ));

    for (j, name) in names.iter().enumerate() {
        let x = grid_x + cell_size * j as f64 + cell_size / 2.0;
        let y = grid_y - 8.0;
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" transform=\"rotate(-45 {:.1} {:.1})\" text-anchor=\"start\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#374151\">{}</text>\n",
            x, y, x, y, name
        ));
    }

    for (i, name) in names.iter().enumerate() {
        let x = grid_x - 8.0;
        let y = grid_y + cell_size * i as f64 + cell_size / 2.0 + 4.0;
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#374151\">{}</text>\n",
            x, y, name
        ));
    }

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

            if i == j {
                out.push_str(&format!(
                    "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#1f2937\" stroke-width=\"1.5\" stroke-dasharray=\"3 2\"/>\n",
                    cx + 2.0, cy + 2.0, cell_size - 4.0, cell_size - 4.0
                ));
            }

            let pct_text = format!("{:.0}%", win_pct * 100.0);
            let text_color = if (win_pct - 0.5).abs() > 0.35 { "#ffffff" } else { "#111827" };
            out.push_str(&format!(
                "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"14\" font-weight=\"700\" fill=\"{}\">{}</text>\n",
                cx + cell_size / 2.0, cy + cell_size / 2.0 + 5.0, text_color, pct_text
            ));
        }
    }

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
    write_svg_file(path, &out);
}
