use std::path::Path;

use super::colors::z_color;
use super::helpers::{svg_header, write_svg_file};

pub fn write_tournament_svg(
    path: &Path,
    teams: &[&str],
    goal_diff_matrix: &[Vec<f64>],
    z_matrix: &[Vec<f64>],
    rankings: &[(usize, f64, f64)],
    games_per_match: usize,
) {
    let n = teams.len();
    if n == 0 { return; }

    let cell_w: f64 = 88.0;
    let cell_h: f64 = 56.0;
    let label_w: f64 = 110.0;
    let label_h: f64 = 70.0;
    let pad_top: f64 = 72.0;
    let pad_left: f64 = 24.0;
    let pad_bottom: f64 = 48.0;
    let standings_w: f64 = 240.0;
    let standings_gap: f64 = 32.0;

    let grid_w = cell_w * n as f64;
    let grid_h = cell_h * n as f64;
    let width = pad_left + label_w + grid_w + standings_gap + standings_w + 16.0;
    let height = pad_top + label_h + grid_h + pad_bottom;

    let grid_x = pad_left + label_w;
    let grid_y = pad_top + label_h;

    let mut zvals: Vec<f64> = vec![];
    for i in 0..n {
        for j in 0..n {
            if i != j { zvals.push(z_matrix[i][j].abs()); }
        }
    }
    zvals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let clamp = zvals.get(zvals.len() * 9 / 10).copied().unwrap_or(5.0).max(2.0);

    let mut out = svg_header(width, height);

    out.push_str(&format!(
        "  <text x=\"{}\" y=\"34\" font-family=\"Arial, sans-serif\" font-size=\"22\" font-weight=\"700\" fill=\"#111827\">V6 Round-robin tournament</text>\n",
        pad_left as i64
    ));
    out.push_str(&format!(
        "  <text x=\"{}\" y=\"54\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#6b7280\">{} games per matchup · {} teams · color = z-score (red=loss, yellow=neutral, green=win)</text>\n",
        pad_left as i64, games_per_match, n
    ));

    for (j, name) in teams.iter().enumerate() {
        let x = grid_x + cell_w * j as f64 + cell_w / 2.0;
        let y = grid_y - 10.0;
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" transform=\"rotate(-40 {:.1} {:.1})\" text-anchor=\"start\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#374151\">{}</text>\n",
            x, y, x, y, name
        ));
    }

    for (i, name) in teams.iter().enumerate() {
        let x = grid_x - 8.0;
        let y = grid_y + cell_h * i as f64 + cell_h / 2.0 + 4.0;
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#374151\">{}</text>\n",
            x, y, name
        ));
    }

    for i in 0..n {
        for j in 0..n {
            let cx = grid_x + cell_w * j as f64;
            let cy = grid_y + cell_h * i as f64;

            if i == j {
                out.push_str(&format!(
                    "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"#f3f4f6\" stroke=\"#d1d5db\" stroke-width=\"1\"/>\n",
                    cx, cy, cell_w, cell_h
                ));
                out.push_str(&format!(
                    "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"18\" fill=\"#9ca3af\">—</text>\n",
                    cx + cell_w / 2.0, cy + cell_h / 2.0 + 6.0
                ));
            } else {
                let z = z_matrix[i][j];
                let gd = goal_diff_matrix[i][j];
                let color = z_color(z, clamp);
                let text_color = if z.abs() > clamp * 0.6 { "#ffffff" } else { "#111827" };

                out.push_str(&format!(
                    "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\" stroke=\"#ffffff\" stroke-width=\"1.5\"/>\n",
                    cx, cy, cell_w, cell_h, color
                ));
                out.push_str(&format!(
                    "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"15\" font-weight=\"700\" fill=\"{}\">{:+.0}</text>\n",
                    cx + cell_w / 2.0, cy + cell_h / 2.0 - 1.0, text_color, gd
                ));
                out.push_str(&format!(
                    "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"10\" fill=\"{}\" opacity=\"0.85\">z={:+.1}</text>\n",
                    cx + cell_w / 2.0, cy + cell_h / 2.0 + 14.0, text_color, z
                ));
            }
        }
    }

    let sx = grid_x + grid_w + standings_gap;
    let sy = grid_y;
    out.push_str(&format!(
        "  <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Arial, sans-serif\" font-size=\"14\" font-weight=\"700\" fill=\"#111827\">Standings</text>\n",
        sx, sy - 12.0
    ));
    out.push_str(&format!(
        "  <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#6b7280\">#  Team</text>\n",
        sx, sy + 4.0
    ));
    out.push_str(&format!(
        "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#6b7280\">Δgoals   z-sum</text>\n",
        sx + standings_w - 8.0, sy + 4.0
    ));
    out.push_str(&format!(
        "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#e5e7eb\" stroke-width=\"1\"/>\n",
        sx, sy + 8.0, sx + standings_w, sy + 8.0
    ));

    let row_h = 28.0;
    for (rank, (team_idx, total_diff, total_z)) in rankings.iter().enumerate() {
        let ry = sy + 20.0 + rank as f64 * row_h;
        let rank_num = rank + 1;

        let badge_color = match rank_num {
            1 => "#d97706",
            2 => "#6b7280",
            3 => "#92400e",
            _ => "#e5e7eb",
        };
        let badge_text_color = if rank_num <= 3 { "#ffffff" } else { "#374151" };
        out.push_str(&format!(
            "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"20\" height=\"18\" rx=\"3\" fill=\"{}\"/>\n",
            sx, ry - 13.0, badge_color
        ));
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Arial, sans-serif\" font-size=\"11\" font-weight=\"700\" fill=\"{}\">{}</text>\n",
            sx + 10.0, ry - 0.5, badge_text_color, rank_num
        ));
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Arial, sans-serif\" font-size=\"12\" fill=\"#111827\">{}</text>\n",
            sx + 26.0, ry, teams[*team_idx]
        ));
        let diff_color = if *total_diff >= 0.0 { "#16a34a" } else { "#dc2626" };
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"Arial, sans-serif\" font-size=\"12\" font-weight=\"700\" fill=\"{}\">{:+.0}</text>\n",
            sx + standings_w - 70.0, ry, diff_color, total_diff
        ));
        out.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"Arial, sans-serif\" font-size=\"11\" fill=\"#6b7280\">{:+.1}</text>\n",
            sx + standings_w - 8.0, ry, total_z
        ));
        if rank < rankings.len() - 1 {
            out.push_str(&format!(
                "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#f3f4f6\" stroke-width=\"1\"/>\n",
                sx, ry + 8.0, sx + standings_w, ry + 8.0
            ));
        }
    }

    out.push_str("</svg>\n");
    write_svg_file(path, &out);
}
