//! Renderar lagets föredragna positioner som SVG (pitch + spelarbubblor).
//! Detta är en ren funktion: TeamPolicyV6 → SVG-fil. Engine-fri.

use std::path::Path;

use training_engine::policy::{TeamPolicyV6, V6Params};

pub fn compute_v6_preferred_xy(params: &V6Params, own_goal_x: f32) -> (f32, f32) {
    let target_y = params.spatial.side.preferred.clamp(20.0, 500.0);
    let dy = target_y - 260.0;
    let pref = params.spatial.own_goal.preferred;
    let dx_sq = pref * pref - dy * dy;
    let target_x = if dx_sq > 0.0 {
        own_goal_x + dx_sq.sqrt()
    } else {
        own_goal_x + pref
    };
    (target_x.clamp(20.0, 860.0), target_y)
}

pub fn write_team_layout_svg(out_path: &Path, team_name: &str, team_desc: &str, params: &TeamPolicyV6) {
    let mut svg = String::new();
    svg.push_str(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 880 660" font-family="ui-monospace,Menlo,monospace">"##);
    svg.push_str(r##"<rect width="880" height="520" fill="#2a6318"/>"##);
    for i in 0..11 {
        let opacity = if i % 2 == 0 { "0.06" } else { "0.025" };
        let fill = if i % 2 == 0 { "black" } else { "white" };
        svg.push_str(&format!(r##"<rect x="{}" y="0" width="80" height="520" fill="{}" fill-opacity="{}"/>"##, i*80, fill, opacity));
    }
    svg.push_str(r##"<rect x="18" y="8" width="844" height="504" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<line x1="440" y1="8" x2="440" y2="512" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<circle cx="440" cy="260" r="62" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<circle cx="440" cy="260" r="3" fill="rgba(255,255,255,0.8)"/>"##);
    svg.push_str(r##"<rect x="18" y="172" width="106" height="176" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<rect x="756" y="172" width="106" height="176" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<rect x="18" y="214" width="54" height="92" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<rect x="808" y="214" width="54" height="92" fill="none" stroke="rgba(255,255,255,0.82)" stroke-width="2"/>"##);
    svg.push_str(r##"<rect x="-8" y="195" width="26" height="130" fill="rgba(255,255,255,0.1)" stroke="rgba(255,255,255,0.9)" stroke-width="3"/>"##);
    svg.push_str(r##"<rect x="862" y="195" width="26" height="130" fill="rgba(255,255,255,0.1)" stroke="rgba(255,255,255,0.9)" stroke-width="3"/>"##);

    let slots = ["FWD", "MID-T", "MID-B", "DEF", "GK"];
    let colors = ["#ff6b35", "#5b9bff", "#5b9bff", "#84cc16", "#fbbf24"];
    let own_goal_x = 18.0_f32;

    for i in 0..5 {
        let s = &params[i].spatial;
        svg.push_str(&format!(r##"<circle cx="{}" cy="260" r="{:.0}" fill="none" stroke="{}" stroke-width="1" stroke-opacity="0.18" stroke-dasharray="4,4"/>"##,
            own_goal_x, s.own_goal.min, colors[i]));
        svg.push_str(&format!(r##"<circle cx="{}" cy="260" r="{:.0}" fill="none" stroke="{}" stroke-width="1" stroke-opacity="0.18" stroke-dasharray="4,4"/>"##,
            own_goal_x, s.own_goal.max, colors[i]));
    }

    for i in 0..5 {
        let s = &params[i].spatial;
        let (px, py) = if i == 4 {
            (training_engine::constants::FIELD_LINE + 21.0, 260.0)
        } else {
            compute_v6_preferred_xy(&params[i], own_goal_x)
        };

        svg.push_str(&format!(r##"<line x1="{:.0}" y1="{:.0}" x2="{:.0}" y2="{:.0}" stroke="{}" stroke-width="1" stroke-opacity="0.22" stroke-dasharray="2,3"/>"##,
            px, s.side.min, px, s.side.max, colors[i]));

        let ball_r = s.ball.preferred.clamp(20.0, 200.0);
        svg.push_str(&format!(r##"<circle cx="{:.0}" cy="{:.0}" r="{:.0}" fill="{}" fill-opacity="0.05" stroke="{}" stroke-width="1" stroke-opacity="0.25"/>"##,
            px, py, ball_r, colors[i], colors[i]));

        let opp_r = s.opponent.preferred.clamp(15.0, 200.0);
        svg.push_str(&format!(r##"<circle cx="{:.0}" cy="{:.0}" r="{:.0}" fill="none" stroke="{}" stroke-width="2" stroke-opacity="0.5" stroke-dasharray="6,3"/>"##,
            px, py, opp_r, colors[i]));

        svg.push_str(&format!(r##"<circle cx="{:.0}" cy="{:.0}" r="13" fill="{}" stroke="white" stroke-width="2.5"/>"##,
            px, py, colors[i]));
        svg.push_str(&format!(r##"<text x="{:.0}" y="{:.0}" fill="white" font-size="10" font-weight="bold" text-anchor="middle">{}</text>"##,
            px, py + 3.5, slots[i]));
    }

    svg.push_str(&format!(r##"<text x="10" y="22" fill="rgba(255,255,255,0.95)" font-size="16" font-weight="bold">{}</text>"##, team_name));
    svg.push_str(&format!(r##"<text x="10" y="38" fill="rgba(255,255,255,0.7)" font-size="11">{}</text>"##, team_desc));
    svg.push_str(r##"<text x="870" y="22" fill="rgba(255,255,255,0.55)" font-size="9" text-anchor="end">solid: preferred · faint: own_goal min/max · dashed ring: opponent pref</text>"##);

    svg.push_str(r##"<rect y="520" width="880" height="140" fill="#0a0a0a"/>"##);
    svg.push_str(r##"<text x="10" y="538" fill="white" font-size="11" font-weight="bold">Spatial preferences (min / preferred / max)</text>"##);
    svg.push_str(r##"<text x="10" y="556" fill="rgba(255,255,255,0.55)" font-size="9">slot     own_goal              side                 ball                 teammate           opponent</text>"##);
    let header_y = 575.0;
    for i in 0..5 {
        let s = &params[i].spatial;
        let y = header_y + (i as f32) * 14.0;
        let txt = format!(
            "{:<6}  {:>4.0}/{:>4.0}/{:>4.0}     {:>4.0}/{:>4.0}/{:>4.0}      {:>4.0}/{:>4.0}/{:>4.0}      {:>4.0}/{:>4.0}/{:>4.0}    {:>4.0}/{:>4.0}/{:>4.0}",
            slots[i],
            s.own_goal.min, s.own_goal.preferred, s.own_goal.max,
            s.side.min, s.side.preferred, s.side.max,
            s.ball.min, s.ball.preferred, s.ball.max,
            s.teammate.min, s.teammate.preferred, s.teammate.max,
            s.opponent.min, s.opponent.preferred, s.opponent.max,
        );
        svg.push_str(&format!(r##"<text x="10" y="{:.0}" fill="{}" font-size="10" xml:space="preserve">{}</text>"##,
            y, colors[i], txt));
    }
    svg.push_str("</svg>");
    let _ = std::fs::write(out_path, svg);
}
