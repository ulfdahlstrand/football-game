# Handoff — WASM-migration (omtag)

**Skapad:** 2026-05-07
**Status:** Branch `refactor/wasm-engine` återskapad från riktiga `main` (b5dc4ba). Inga ändringar gjorda än. Inget committat.

---

## Vad som hände i föregående session

Jag (claude-code) påbörjade WASM-migreringen men **branchade från outdated lokal `main` som låg 35 commits efter `origin/main`**. Det första `git checkout main` skedde innan `git pull`, så jag jobbade hela sessionen mot en gammal kodbas där:

- `team-select.jsx`, `match-hud.jsx`, `match-summary.jsx`, `sound-engine.js` saknades
- `football-match.jsx` var 1115 rader (gammal version) istället för 2058
- `training-engine/` saknade V6-grejer (brain.rs, spatial.rs, svg.rs, set-pieces, GK-state, free-kicks, slow_timer, per-player stats m.m.)

Användaren upptäckte detta när team selectorn var borta. Jag stash:ade allt, fast-forwardade `main`, raderade gamla `refactor/wasm-engine` och skapade om branchen från verkliga `main` (b5dc4ba — som inkluderar v6-only refaktorn).

**Working tree är nu RENT på korrekt main.** Inga ändringar gjorda i detta tillstånd.

---

## Vad som finns sparat som referens i `/tmp/wasm-old/`

Mitt gamla (delvis obsoleta) WASM-arbete:

| Fil | Användbarhet |
|-----|--------------|
| `match-engine-wasm/Cargo.toml` | ✅ Återanvändbar nästan rakt av — bara path-dep till training-engine + wasm-bindgen + serde + rand + getrandom(js) |
| `match-engine-wasm/src/lib.rs` | ⚠️ Måste skrivas om — använder `PolicyParams` (gamla V1 API) men den nya engine använder `TeamPolicyV6` med `V6Params` per spelare. Strukturen (thread_local SESSIONS, GameSession wrapper, build_state_json, HumanInput) är dock användbar som mall. |
| `Cargo.toml.workspace` | ✅ Användbar rakt av — root workspace med training-engine + match-engine-wasm |
| `training-engine-lib.rs` | ⚠️ Behöver utökas — gamla versionen gated bara `trainer` + `session` bakom wasm32. Den nya engine har `brain`, `spatial`, `svg` också (svg behöver gates eftersom den skriver filer). |
| `pkg/` | ❌ Ej användbar — byggd från gammal kod |
| `football-match.jsx.old` | ❌ Ej användbar — överskriver den rika 2058-radersversionen som har team-select-integration, set-piece UI, sound-effekter osv. |

---

## Nuvarande tillstånd (verifiera först!)

```bash
git status                    # Ska vara: clean
git log --oneline -1          # Ska vara: b5dc4ba Merge pull request #3 from ulfdahlstrand/refactor/v6-only
git rev-parse --abbrev-ref HEAD  # Ska vara: refactor/wasm-engine
ls *.jsx *.js | head -20      # Ska visa: team-select.jsx, match-hud.jsx, match-summary.jsx, sound-engine.js m.fl.
wc -l football-match.jsx       # Ska vara: 2058
wc -l training-engine/src/*.rs # Bör vara: 7584 totalt (inkl. ai 828, brain 66, game 257, main 2279, physics 543, policy 298, session 205, spatial 212, svg 622, trainer 170, constants 46)
```

Om något skiljer — något har gått fel; gå inte vidare innan du förstår varför.

---

## Mål (oförändrat från ursprungliga handoffen)

Eliminera duplicerad matchlogik mellan inline-simuleringen i `football-match.jsx` (~1500 rader spel-logik blandat med rendering) och `training-engine/src/*.rs` genom att kompilera Rust-motorn till WASM och låta browsern köra exakt samma motor som tränar AI:n.

**Slutresultat:** En källa till sanning (Rust). JS = renderingslager + input + UI-glue. Ingen sync-drift.

---

## Den nya kodbasen — vad du jobbar mot

### Rust-motorn (träning)

`training-engine/src/`:

```
constants.rs   46 rader   — fältgeometri, fysik
game.rs       257 rader   — Game struct (rich), Player med brain+stats+gk_dive+slow_timer+penalties_*, Stats med fouls/free_kicks/corners/penalties, Phase, Role, PlayerState, Ball, make_players, effective_policy, CLUSTER_START static
brain.rs       66 rader   — PlayerBrain::V6(V6Params), tick_player → ai::v6_tick
policy.rs     298 rader   — PolicyParams (8 klassiska fält), V6Params (spatial + decisions + gk), V6Spatial, V6Decisions, GkDecisionParams, TeamPolicyV6 = [V6Params; 5]
ai.rs         828 rader   — v6_tick (huvudbeslut), cpu_find_pass, många helpers
physics.rs    543 rader   — step_game, set pieces (free kick, corner, kick-in, goal-kick), penalty, GK-state, knock_player, tackle_player, do_shoot, slow_player, attribute_goal
spatial.rs    212 rader   — V6 spatial cost-funktioner (DistancePref m.m.)
trainer.rs    170 rader   — rayon-paralleliserad evaluation (måste cfg-gates för wasm32)
session.rs    205 rader   — fs-baserad I/O (TeamBaselineFileV6) — cfg-gate hela för wasm32
svg.rs        622 rader   — SVG-generering, fs-output — cfg-gate hela för wasm32
main.rs      2279 rader   — CLI, alla träningskommandon (oförändrad — bin-only)
```

**Game struct (game.rs):** har INTE `human_player: Option<usize>` ännu. Detta måste läggas till för att WASM ska kunna skippa AI för spelare 0.

**`physics::step_game`** loopar igenom alla spelare och anropar `crate::brain::tick_player(game, i, rng)`. För human-stöd: lägg till skip om `game.human_player == Some(game.pl[i].id)`.

**Cargo.toml:** har rayon i `[dependencies]` — behöver flyttas till `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`. Och `[lib]` target behöver läggas till (just nu bara `[[bin]]`).

### Frontend (browser)

`Fotbolls-RPG.html` laddar i denna ordning:
```
react + react-dom + babel-standalone (CDN UMD)
sound-engine.js
tweaks-panel.jsx · questions.jsx · game-world.jsx · match-screen.jsx · binder.jsx
team-select.jsx · match-hud.jsx · match-summary.jsx · football-match.jsx · app.jsx
```

`football-match.jsx` — **2058 rader**, en monolitisk fil med:
- All sprite-ritning (drawKnocked, drawSpriteDown/Up/Side, drawPlayer ca rad 56–186)
- Hela inline-engine: `newGame`, `doShoot`, `tacklePlayer`, `knockPlayer`, `cpuTick`, `v6Tick` (rad ~917), `baselineCpuTick`, `cpuFindPass`, `applyPolicyToTeam`, `setBallOwner`, `awardSetPiece`, `startFreeKick`, `restartGoalKick/KickIn/Corner`, `startPenalty`, `handleBallOut`, GK-logik, slow-timer m.m.
- `FootballMatch` React-komponent (rad ~1294 → slut): canvas-loop med `update()` (~rad 1497) + `draw()` (~rad 1767), keyboard input, integration med `window.TeamSelectScreen`, opponent-fetch från `data/policies/opponents.json`, sound calls (`window.SFX`), set-piece overlays, mega-shot, celebration, statistik, fulltime-summary
- Statistik-tracking (g._stats med possOwnHalf, possOppHalf, _possOwnFrames m.m.)

**Det stora problemet:** simuleringen är djupt sammanflätad med rendering+UI-state. Att bryta ut den kräver kirurgi.

`team-select.jsx` (641 rader, separat) — `window.TeamSelectScreen`. Visar lagval med flaggor/färger, `LineupScreen` för formationer, hämtar `data/teams/<slug>/roster.json`. Sätter team config på `g` innan match startar.

`match-hud.jsx`, `match-summary.jsx`, `sound-engine.js` — UI/audio, inte simulering. Lämna i fred.

---

## Plan (rekommenderad approach)

### Fas 1 — Rust + WASM-infrastruktur (mekaniskt, lågrisk)

1. **Root `Cargo.toml`** (skapa):
   ```toml
   [workspace]
   members = ["training-engine", "match-engine-wasm"]
   resolver = "2"
   ```

2. **`training-engine/Cargo.toml`** — gör rayon plattformsbetingad + lägg till `[lib]`:
   ```toml
   [[bin]]
   name = "training-engine"
   path = "src/main.rs"

   [lib]
   name = "training_engine"
   path = "src/lib.rs"

   [dependencies]
   rand       = { version = "0.8", features = ["small_rng"] }
   rand_distr = "0.4"
   serde      = { version = "1", features = ["derive"] }
   serde_json = "1"
   anyhow     = "1"

   [target.'cfg(not(target_arch = "wasm32"))'.dependencies]
   rayon      = "1"
   ```

3. **`training-engine/src/lib.rs`** (ny):
   ```rust
   pub mod constants;
   pub mod game;
   pub mod brain;
   pub mod policy;
   pub mod spatial;
   pub mod ai;
   pub mod physics;
   #[cfg(not(target_arch = "wasm32"))]
   pub mod trainer;
   #[cfg(not(target_arch = "wasm32"))]
   pub mod session;
   #[cfg(not(target_arch = "wasm32"))]
   pub mod svg;
   ```
   
   OBS: `main.rs` har `mod constants; mod game; ...` deklarationer — de fungerar fortfarande för bin-targeten (binär kompileras separat). Inga ändringar i main.rs.

4. **`training-engine/src/trainer.rs`** — cfg-gate rayon på två ställen:
   ```rust
   #[cfg(not(target_arch = "wasm32"))]
   use rayon::prelude::*;
   
   // I evaluate_policies(), runt rad 118:
   #[cfg(not(target_arch = "wasm32"))]
   let chunk_results: Vec<_> = seeds.into_par_iter().map(|(seed, swap)| run_one_game(...)).collect();
   #[cfg(target_arch = "wasm32")]
   let chunk_results: Vec<_> = seeds.into_iter().map(|(seed, swap)| run_one_game(...)).collect();
   ```

5. **`training-engine/src/game.rs`** — lägg till `pub human_player: Option<usize>` i `Game` struct + initialisera till `None` i `Game::new`.

6. **`training-engine/src/physics.rs::step_game`** — i player-loopen, skip om human:
   ```rust
   if game.human_player == Some(game.pl[i].id) { continue; }
   crate::brain::tick_player(game, i, rng);  // OBS: heter brain::tick_player nu, inte ai::baseline_cpu_tick
   ```
   Verifiera nuvarande loop runt rad 367 — den anropar förmodligen `crate::brain::tick_player` redan.

7. **Verifiera:** `cd training-engine && cargo build --release` — ska vara grön.

8. **`match-engine-wasm/Cargo.toml`** (ny):
   ```toml
   [package]
   name = "match-engine-wasm"
   version = "0.1.0"
   edition = "2021"

   [lib]
   crate-type = ["cdylib"]

   [dependencies]
   wasm-bindgen = "0.2"
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   rand = { version = "0.8", features = ["small_rng"] }
   getrandom = { version = "0.2", features = ["js"] }
   training-engine = { path = "../training-engine" }
   ```

9. **`match-engine-wasm/src/lib.rs`** — använd /tmp/wasm-old/match-engine-wasm/src/lib.rs som mall, men:
   - Ta in `TeamPolicyV6` istället för `PolicyParams` i `create_game` (parsa baseline.json som `TeamBaselineFileV6` med `playerParams: [V6Params; 5]`)
   - Anropa `Game::for_team_battle_v6(&team0, &team1)` istället för `Game::new(p0, p1)`
   - Sätt `game.human_player = Some(0)` direkt efter
   - Step-loopen — `physics::step_game(&mut game, rng)` (oförändrad)
   - Output-state: utöka JsPlayer/JsBall/JsGameState att matcha vad den nya football-match.jsx draw() läser (inkl. slow_timer, gk_dive_*, set_piece_x/y, free_kick_active etc.)
   - Render-state (facing, stepCounter, celebrate_timer) som GameSession-fält som tidigare

10. **Bygg:** `wasm-pack build match-engine-wasm --target no-modules --release --out-dir ../pkg`. Förvänta dig ~150KB wasm.

11. **`.gitignore`** — lägg till `pkg/` och `target/`.

### Fas 2 — Frontend-integration (svår, kräver kirurgi)

Detta är det komplicerade steget. Den nya `football-match.jsx` (2058 rader) har simuleringen djupt integrerad med:
- `g._stats._possOwnFrames` (statistik-frames-räknare)
- `window.SFX` ljudtrigger på pass/skott/mål
- Set-piece text-overlays
- Mega-shot mekanik
- Celebration-animationer
- Tackle-animationer
- GK-dive-animationer
- Statistik per spelare (för match-summary.jsx)
- Integration med `window.TeamSelectScreen`

**Strategi:**

A. Studera `update()` i football-match.jsx (rad ~1497) för att se vad den modifierar i `g`. Detta är vad WASM måste producera.

B. Studera `draw()` (rad ~1767) för att se vilka fält den läser. Detta är vad WASM-output måste innehålla.

C. Replace `update()` med en WASM-baserad version som:
   - Läser `keysRef.current` + pendingActions
   - Bygger input-objekt
   - Anropar `wasm_bindgen.step_game(handle, JSON.stringify(input))`
   - Parsar resultatet och uppdaterar `gRef.current`
   - Triggar ljud via `window.SFX` baserat på event-flaggor som WASM exponerar (t.ex. `events: { goalScored, ballKicked, tackleCompleted }`)
   
D. **Behåll i JS:** sound triggers, set-piece text mappning (svenska strängar), team-select integration, stats-frame-räknare för possession-overlay (kan beräknas från frames i JS), summary-data extraktion vid fulltime.

E. **Ta bort från JS:** newGame, doShoot, tacklePlayer, knockPlayer, cpuTick, v6Tick, baselineCpuTick, cpuFindPass, applyPolicyToTeam, setBallOwner, awardSetPiece, startFreeKick, restartGoalKick/KickIn/Corner, startPenalty, handleBallOut, alla physics-helpers. Spara i en separat dold fil eller bara radera.

**Tips:** Ta inte bort allt på en gång. Lämna inline-engine kvar som dead code först, lägg till WASM-anropet parallellt, jämför outputs sida vid sida i en debug-overlay tills du är säker. Ta sedan bort den döda koden.

### Fas 3 — Cleanup (efter allt fungerar)

Radera:
- `match-engine.js` (UMD-modul, bara använd av Node CLI)
- `train-policy.js` (ersatt av Rust-träning)
- `simulate-match.js` (ersätt med direkt anrop till `training-engine` binary, eller bygg WASM med `--target nodejs` och uppdatera scriptet)
- `data/policies/candidate.json` om den fortfarande är föräldralös

---

## Gotchas / fallgropar att undvika

1. **`git checkout main && git checkout -b ny-branch` utan `git pull` först** — det är så detta kraschade. Verifiera ALLTID att `main` är i nivå med `origin/main` innan du branchar.

2. **`Edit`-tool på TOML-filer kan röra fler sektioner än du tror.** När jag flyttade `rayon = "1"` till platform-conditional flyttades även `serde`, `rand_distr`, `anyhow` av misstag eftersom min `old_string` matchade slutet av `[dependencies]`-blocket. Verifiera Cargo.toml efter varje edit.

3. **getrandom kräver `features = ["js"]`** för wasm32-unknown-unknown. Annars: "the wasm*-unknown-unknown targets are not supported by default".

4. **training-engine var bin-only.** För att match-engine-wasm ska kunna `path = "../training-engine"` behövs `[lib]` target med `name = "training_engine"` (underscore!) + `src/lib.rs`.

5. **session.rs och svg.rs använder `std::fs`.** Måste cfg-gates `#[cfg(not(target_arch = "wasm32"))]` i lib.rs.

6. **`PolicyParams` vs `V6Params`:** baseline.json i den nya kodbasen är `TeamBaselineFileV6 { playerParams: [V6Params; 5] }`. Inte `BaselineFile { parameters: PolicyParams }` som i gamla kodbasen. Min gamla WASM-lib.rs använder fel typ.

7. **`Game::for_team_battle_v6` är vad du vill anropa**, inte `Game::new` (som bara sätter team-policies utan att sätta brain på spelarna).

8. **wasm-pack profiler ignoreras med workspace.** Sätt `[profile.release]` i ROOT Cargo.toml om du vill ha custom optimering, inte i sub-craten. Men default fungerar fint.

9. **`async useEffect` är lurigt.** Använd IIFE-pattern + `cancelled` flag för cleanup när komponenten unmountar innan WASM-init är klar.

10. **`--target no-modules`** är rätt val här eftersom Fotbolls-RPG.html inte använder bundler — bara CDN UMD + Babel standalone. Andra targets (`web`, `bundler`, `nodejs`) kräver ES modules eller npm pipeline.

---

## Användarinstruktioner i ord

> "vi har precis mergat uppstädning av vår engine"
> "det finns en plan för hur vi ska ersätta jsmotorn med en WASM compilering av vår rust motor"
> "jag vill att vi skapar en egen branch för detta"
> "när vi är klara har vi ingen js motor utan det är istället en kompilerad version av rust motorn"
> "målet med detta är att js delarna enbart skall vara ett tunt renderingslager ovanpå wasm filen så att exakt samma motor används i träningen som i web spelet"
> "tar bort allt som har med själva js motorn att göra som inte behövs"
> (efter att ha upptäckt att team selectorn var borta:) "varför är hela team selectorn borta? även flaggor och allt sånt vi gjort är borta men ligger i main branchen"

Användarens prioriteringar (i ordning):
1. **Behåll team-select, flags, sound, HUD, summary** — allt fanns i main, ska INTE försvinna
2. JS = tunt renderingslager
3. Rust-motorn (samma kod som tränar) ska köra i browsern via WASM
4. Inga gamla JS-motor-rester kvar i slutet

---

## Komma igång

```bash
cd /Users/ulfdahlstrand/Projects/Code/Private/Spel/football-game

# Verifiera utgångsläge
git status                                    # ska vara clean
git log --oneline -1                          # ska vara b5dc4ba
ls team-select.jsx match-hud.jsx              # ska finnas

# Kolla referensfiler
ls /tmp/wasm-old/

# Börja med Fas 1, steg 1 (workspace Cargo.toml)
```

Lycka till. Den hårda biten är football-match.jsx-kirurgin — ta tid på att förstå update() och draw() innan du rör dem.

— claude-code (sonnet 4.6 / opus 4.7)
