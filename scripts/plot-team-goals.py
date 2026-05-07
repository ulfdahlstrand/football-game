#!/usr/bin/env python3
"""Plot champion goals/match progression across all anneal stages for a team."""
import json, sys, glob, os

def read_stage(path):
    with open(path) as f:
        return json.load(f)

def main(team_name):
    team_dir = f"/Users/ulfdahlstrand/Projects/Code/Private/Spel/football-game/data/teams/{team_name}"
    summaries = sorted(glob.glob(f"{team_dir}/sessions/*/summary.json"))
    if not summaries:
        print(f"No summaries found for {team_name}")
        sys.exit(1)

    # Collect all (cumulative_epoch, champion_avg_goals) — only accepted epochs
    points = []
    cum = 0
    stage_boundaries = [(0, "start")]
    for spath in summaries:
        s = read_stage(spath)
        stage_label = os.path.basename(os.path.dirname(spath)).replace("anneal-stage-", "S")
        stage_start_cum = cum
        for h in s["history"]:
            cum += 1
            if h.get("accepted"):
                points.append((cum, h["candidateAvgGoals"], stage_label))
        # Stage boundary
        stage_boundaries.append((cum, stage_label))

    if not points:
        print("No accepted epochs")
        sys.exit(1)

    # SVG dimensions
    W, H = 1100, 360
    PAD_L, PAD_R, PAD_T, PAD_B = 60, 20, 30, 50
    plot_w = W - PAD_L - PAD_R
    plot_h = H - PAD_T - PAD_B

    max_x = points[-1][0]
    max_y = max(p[1] for p in points) * 1.1
    min_y = 0

    def x_of(epoch): return PAD_L + plot_w * epoch / max_x
    def y_of(g): return PAD_T + plot_h * (1 - (g - min_y) / (max_y - min_y))

    svg = []
    svg.append(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" font-family="ui-monospace,Menlo,monospace">')
    svg.append(f'<rect width="{W}" height="{H}" fill="#0a0a0a"/>')
    svg.append(f'<text x="10" y="22" fill="white" font-size="14" font-weight="bold">{team_name} — champion avg goals/match (accepted epochs)</text>')

    # Y-axis grid + labels
    for i in range(6):
        gv = max_y * i / 5
        y = y_of(gv)
        svg.append(f'<line x1="{PAD_L}" y1="{y}" x2="{W-PAD_R}" y2="{y}" stroke="#222" stroke-width="1"/>')
        svg.append(f'<text x="{PAD_L-8}" y="{y+3}" fill="rgba(255,255,255,0.6)" font-size="10" text-anchor="end">{gv:.1f}</text>')

    # Stage boundaries (vertical lines)
    stage_colors = ["#5b9bff", "#84cc16", "#fbbf24", "#ff6b35"]
    for i, (cb, label) in enumerate(stage_boundaries[1:]):
        if cb > 0 and cb < max_x:
            x = x_of(cb)
            svg.append(f'<line x1="{x}" y1="{PAD_T}" x2="{x}" y2="{H-PAD_B}" stroke="{stage_colors[i % 4]}" stroke-width="1" stroke-opacity="0.4" stroke-dasharray="3,3"/>')

    # Plot line connecting accepted epochs
    pts_str = " ".join(f"{x_of(p[0]):.1f},{y_of(p[1]):.1f}" for p in points)
    svg.append(f'<polyline points="{pts_str}" fill="none" stroke="#5b9bff" stroke-width="1.5" stroke-opacity="0.7"/>')

    # Plot dots colored by stage
    stage_to_color = {}
    for i, (cb, label) in enumerate(stage_boundaries[1:]):
        stage_to_color[label] = stage_colors[i % 4]

    for (epoch, gval, label) in points:
        c = stage_to_color.get(label, "white")
        svg.append(f'<circle cx="{x_of(epoch):.1f}" cy="{y_of(gval):.1f}" r="2" fill="{c}" fill-opacity="0.8"/>')

    # X-axis label
    svg.append(f'<text x="{W/2}" y="{H-12}" fill="rgba(255,255,255,0.7)" font-size="11" text-anchor="middle">cumulative epoch (across stages)</text>')
    svg.append(f'<text x="14" y="{H/2}" fill="rgba(255,255,255,0.7)" font-size="11" text-anchor="middle" transform="rotate(-90 14 {H/2})">avg goals/match</text>')

    # Final value annotation
    last = points[-1]
    svg.append(f'<text x="{W-PAD_R-4}" y="{y_of(last[1])-6}" fill="white" font-size="11" text-anchor="end" font-weight="bold">{last[1]:.2f}</text>')

    # Stage legend
    svg.append(f'<text x="10" y="{H-32}" fill="rgba(255,255,255,0.7)" font-size="10">stages: ')
    x = 70
    for label, c in stage_to_color.items():
        svg.append(f'<tspan fill="{c}">●</tspan> <tspan fill="rgba(255,255,255,0.85)">{label}</tspan>  ')
    svg.append('</text>')

    # Stats footer
    n_accepts = len(points)
    svg.append(f'<text x="10" y="{H-50}" fill="rgba(255,255,255,0.6)" font-size="10">{n_accepts} accepted epochs · max {max_y/1.1:.2f} goals/match · final {last[1]:.2f}</text>')

    svg.append('</svg>')
    out = f"{team_dir}/goals-progression.svg"
    with open(out, "w") as f:
        f.write("\n".join(svg))
    print(f"Wrote {out}")
    print(f"  {n_accepts} accepted epochs across {len(summaries)} stages")
    print(f"  goals/match: start={points[0][1]:.2f} → final={last[1]:.2f}")

if __name__ == "__main__":
    team = sys.argv[1] if len(sys.argv) > 1 else "tempest-united"
    main(team)
