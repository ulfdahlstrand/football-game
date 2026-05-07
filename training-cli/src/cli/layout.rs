//! CLI-orchestrering för layout-rendering. Själva renderingen ligger i
//! `training_render::team_layout` — denna modul anropar bara den.

use std::path::Path;

use training_engine::session::read_team_baseline_v6;
use training_render::write_team_layout_svg;

pub fn regenerate_all_team_svgs(project_root: &Path) {
    let teams_dir = project_root.join("data").join("teams");
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(&teams_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let baseline = path.join("baseline.json");
            if !baseline.exists() { continue; }
            let team_name = path.file_name().unwrap().to_string_lossy().into_owned();
            let baseline_file = match read_team_baseline_v6(&baseline) {
                Ok(b) => b, Err(e) => { eprintln!("  ! {}: {}", team_name, e); continue; }
            };
            let display = baseline_file.description.clone().unwrap_or_else(|| team_name.clone());
            write_team_layout_svg(&path.join("layout.svg"), &team_name, &display, &baseline_file.player_params);
            println!("  ✓ {} → {}/layout.svg", team_name, path.display());
            count += 1;
        }
    }
    println!("Regenerated {} team layout SVGs", count);
}
