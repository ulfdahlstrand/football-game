# Handoff — WASM-migrering av matchmotorn

**Skapad:** 2026-05-07
**Föregående session:** v6-only refactor (branch `refactor/v6-only`)
**Nästa session:** WASM-migrering (egen branch, egen commit)

---

## Var vi står

Branch `refactor/v6-only` har 3 commits redo att mergas/pushas:
1. `chore(data): archive training statistics before v1-v4 cleanup`
2. `chore(data): remove v1-v4 training data and legacy matrices`
3. `refactor: remove v1-v4 code, v6 is now the only version`

Kodbasen är nu **v6-only**. All v1–v4-logik är borta från både Rust och JS.

**Verifierat fungerande:** `cargo build --release` grön. Score-probe `nebula-rangers vs aurora-fc` på 30 matcher gav 28W/1D/1L.

---

## Mål för WASM-sessionen

Eliminera duplicerad matchlogik mellan `match-engine.js` (browser) och `training-engine/src/*.rs` (träning) genom att kompilera Rust-motorn till WASM och använda samma kod på båda sidor.

**Drivkraft:** En källa, ingen sync-drift, paving the way för production-deploy.

---

## Arkitekturskiss

```
training-engine/                  ← Rust, native binary för träning (oförändrad)
  src/
    main.rs                       ← CLI, träningskommandon
    game.rs, ai.rs, physics.rs    ← matchsimuleringen
    policy.rs, brain.rs           ← V6-typer
    ...

match-engine-wasm/                ← NY crate som exponerar matchsim till browsern
  Cargo.toml                      ← wasm-bindgen + serde-wasm-bindgen
  src/lib.rs                      ← #[wasm_bindgen] wrappers

pkg/                              ← genereras av wasm-pack (gitignoreras)
  match_engine_wasm.js
  match_engine_wasm_bg.wasm

match-engine.js                   ← ersätts med tunn loader som anropar pkg/
*.jsx                             ← oförändrade om API-ytan bevaras
```

---

## Steg-för-steg

### 1. Skapa WASM-crate
```toml
# match-engine-wasm/Cargo.toml
[package]
name = "match-engine-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
training-engine = { path = "../training-engine" }
```

**Varning:** `training-engine` använder `rayon` i `trainer.rs`. WASM saknar trådar by default. Antingen:
- (a) Splita ut `trainer.rs` till en separat modul som inte kompileras för wasm32, ELLER
- (b) Gate rayon-användning med `#[cfg(not(target_arch = "wasm32"))]`

Sannolikt (a) är renast — träningen behövs ändå inte i browsern.

### 2. Exponera API:et
Browsern behöver minst:
- `createGame(team0_baseline, team1_baseline) -> GameHandle`
- `stepGame(handle) -> GameStateView`
- `runSimulation(team0, team1, seed) -> {score0, score1, stats}` (för snabb sim)

Alla typer som passerar gränsen måste vara `serde`-serialiserbara → använd `serde-wasm-bindgen` för konvertering till/från JS-objekt.

### 3. Bygga
```bash
cd match-engine-wasm
wasm-pack build --target web --out-dir ../pkg
```

### 4. Ersätta `match-engine.js`
Behåll filen som tunn proxy:
```js
import init, * as wasm from './pkg/match_engine_wasm.js';
await init();
export const createGame = wasm.create_game;
// osv. — exportera samma namn som tidigare match-engine.js
```

Eller migrera direkt — uppdatera `*.jsx` till att importera från `pkg/` och ta bort `match-engine.js` helt.

### 5. Verifiera
- `npx http-server . -p 8765 --cors -c-1` och öppna `http://localhost:8765`
- Spela en match — visuellt identisk med tidigare
- Determinism-test: samma seed i WASM-build och native binary ska ge samma slutresultat

---

## Kritiska filer som nästa session måste läsa

| Fil | Varför |
|-----|--------|
| `match-engine.js` | API-ytan som ska bevaras — se vad som exporteras |
| `training-engine/src/game.rs` | `Game::for_team_battle_v6()`, `step_game` |
| `training-engine/src/trainer.rs` | Använder rayon — måste isoleras från WASM-build |
| `training-engine/src/main.rs` | CLI — påverkas inte men bra att veta att den finns |
| `*.jsx` (i synnerhet `football-match.jsx`, `match-screen.jsx`) | Konsumenter av match-engine.js |
| `index.html` | Hur scripts laddas idag (UMD/global vs ES-module) |

Kör `grep -rn "match-engine" --include="*.jsx" --include="*.html" --include="*.js"` för att hitta alla anropsplatser.

---

## Gotchas

1. **Modulformat:** Nuvarande `match-engine.js` är UMD med global `MatchEngine`. WASM-pack genererar ES-moduler. Antingen migrera frontend till ES-moduler eller wrappa i UMD-shim.

2. **Random:** Rust-koden använder `rand::SmallRng` med `SeedableRng::seed_from_u64`. För browser-determinism, exponera explicit seed-parameter.

3. **`std::thread`/rayon i `trainer.rs`:** WASM single-threaded. Splita matchsim (deterministisk, single-threaded) från träning (parallell).

4. **Bundle-storlek:** En naiv WASM-build kan bli 1+ MB. Använd `wasm-opt` (`wasm-pack build --release` gör detta automatiskt) och `[profile.release] opt-level = "s"`.

5. **`baseline.json`-laddning:** Idag fetchas via JS. WASM kan ta in den som JSON-sträng från JS-sidan och deserialisera med serde — undvik fil-I/O i WASM.

6. **Konstanter:** `match-engine.js` exporterar `constants` (FW, FH, etc). Säkerställ att de exponeras från WASM också, eller läs från en delad JSON.

---

## Branch-strategi

```bash
# Utgå från refactor/v6-only (eller main efter merge)
git checkout refactor/v6-only
git pull
git checkout -b refactor/wasm-engine

# Jobba klart
# Verifiera: cargo build, wasm-pack build, browser-test
git add -A && git commit -m "refactor: compile match engine to WASM, eliminate JS/Rust drift"
```

---

## MemPalace-referens

Sessionssummering finns i:
- Diary: `claude-code` / topic `football-game/v6-only-refactor`
- Drawer: wing `football-game` / room `decisions`
- Drawer: wing `football-game` / room `stats`

Sökbar via `mempalace_search "v6-only refactor"`.
