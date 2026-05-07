use std::path::Path;

use super::helpers::{svg_header, write_svg_file};
use training_engine::session::EpochSummary;

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

    let mut out = svg_header(width, height);

    out.push_str(&format!(
        "  <text x=\"{}\" y=\"30\" font-family=\"Arial, sans-serif\" font-size=\"22\" font-weight=\"700\" fill=\"#111827\">Training progress</text>\n",
        pad_left as i64
    ));
    out.push_str(&format!(
        "  <text x=\"{}\" y=\"30\" text-anchor=\"end\" font-family=\"Arial, sans-serif\" font-size=\"13\" fill=\"#4b5563\">Final champion epoch: {}</text>\n",
        (width - pad_right) as i64, final_champion_epoch
    ));
    out.push_str("  <g font-family=\"Arial, sans-serif\">\n");

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

    out.push_str(&format!(
        "    <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#111827\" stroke-width=\"1.4\"/>\n",
        pad_left, pad_top, pad_left, height - pad_bottom
    ));
    out.push_str(&format!(
        "    <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#111827\" stroke-width=\"1.4\"/>\n",
        pad_left, height - pad_bottom, width - pad_right, height - pad_bottom
    ));

    out.push_str(&format!(
        "    <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"13\" fill=\"#111827\">Epoch</text>\n",
        pad_left + plot_w / 2.0, height - 24.0
    ));
    let mid_y = pad_top + plot_h / 2.0;
    out.push_str(&format!(
        "    <text x=\"20\" y=\"{:.1}\" transform=\"rotate(-90 20 {:.1})\" text-anchor=\"middle\" font-size=\"13\" fill=\"#111827\">Goals / goal diff</text>\n",
        mid_y, mid_y
    ));

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

    let legend_y = height - 54.0;
    out.push_str(&format!("    <g transform=\"translate({}, {:.1})\" font-size=\"13\" fill=\"#111827\">\n", pad_left as i64, legend_y));
    out.push_str("      <rect x=\"0\" y=\"-12\" width=\"14\" height=\"4\" fill=\"#2563eb\"/><text x=\"22\" y=\"-7\">Candidate avg goals</text>\n");
    out.push_str("      <rect x=\"190\" y=\"-12\" width=\"14\" height=\"4\" fill=\"#ef4444\"/><text x=\"212\" y=\"-7\">Baseline avg goals</text>\n");
    out.push_str("      <rect x=\"378\" y=\"-12\" width=\"14\" height=\"4\" fill=\"#7c3aed\"/><text x=\"400\" y=\"-7\">Goal diff</text>\n");
    out.push_str("      <circle cx=\"548\" cy=\"-10\" r=\"5\" fill=\"#16a34a\"/><text x=\"562\" y=\"-7\">Accepted improvement</text>\n");
    out.push_str("    </g>\n");

    out.push_str("  </g>\n</svg>\n");
    write_svg_file(path, &out);
}
