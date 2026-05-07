# Football Game — Projektkarta

Ett AI-tränat fotbollsspel med en Rust-träningsmotor, JSX-frontend och ett lagsystem där varje lag har en coach med unik personlighet.

---

## Struktur

```
football-game/
├── CLAUDE.md                    ← du är här
├── index.html                   ← spelets ingångspunkt (öppnas i webbläsare)
├── match-engine.js              ← matchsimulering i JS (används av frontend)
├── simulate-match.js            ← CLI-wrapper för att köra enstaka matcher
├── train-policy.js              ← äldre JS-baserad träning (ersatt av Rust-motorn)
├── sound-engine.js              ← ljud och musik
├── coach_nudge.py               ← verktyg för manuella parameterknuffar (se COACH_RULES)
│
├── *.jsx                        ← React-komponenter för UI
│   ├── app.jsx                  ← rot-app
│   ├── game.jsx                 ← spelflöde
│   ├── game-world.jsx           ← världsvy
│   ├── match-screen.jsx         ← matchvy
│   ├── match-hud.jsx            ← live-HUD under match
│   ├── match-summary.jsx        ← matchsammanfattning
│   ├── football-match.jsx       ← matchlogik-komponent
│   ├── team-select.jsx          ← lagval
│   ├── binder.jsx               ← manager-vy (binder = GM)
│   ├── questions.jsx            ← dialoglager
│   └── tweaks-panel.jsx         ← debug/justeringspanel
│
├── training-engine/             ← Rust-träningsmotor (huvud-AI)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs              ← CLI-kommandon och main
│       ├── trainer.rs           ← träningsloop, annealing, mutation
│       ├── session.rs           ← laddar/sparar baseline.json per lag
│       ├── policy.rs            ← PolicyParams, TeamPolicyV6 — parameterstruktur
│       ├── brain.rs             ← beslutslogik per spelare
│       ├── ai.rs                ← AI-beteende och spelarsimulering
│       ├── game.rs              ← matchsimulering i Rust
│       ├── physics.rs           ← bollrörelser och kollisioner
│       ├── spatial.rs           ← positionsberäkningar
│       ├── svg.rs               ← genererar layout.svg
│       └── constants.rs         ← globala konstanter
│
├── scripts/                     ← hjälpskript
│   ├── log-server.py            ← lokal server för matchloggar
│   ├── compute-team-stats.py    ← statistikberäkning
│   ├── plot-team-goals.py       ← visualisering av måldata
│   └── overnight-train*.sh      ← körscheman för nattlig träning (v3/v4/aktuell)
│
├── match-logs/                  ← sparade matchloggar (JSON per match)
│
└── data/
    ├── teams/                   ← ett lag per mapp
    │   ├── COACH_RULES.md       ← regler som gäller ALLA tränare i ligan
    │   ├── <lag>/
    │   │   ├── coach.md         ← tränarens persona, filosofi, röst & språk
    │   │   ├── coaching.md      ← tränarjournal — historik, aktiv plan, lärdomar
    │   │   ├── baseline.json    ← tränade AI-parametrar (V6 — redigeras INTE direkt)
    │   │   ├── roster.json      ← spelarnamn och roller
    │   │   ├── info.md          ← laginformation, turneringsresultat, beslutsparametrar
    │   │   ├── layout.svg       ← visuell representation av lagets positionering
    │   │   └── logo.svg         ← laglogotyp
    │   │
    │   └── [lag i ligan]
    │       aurora-fc · eclipse-town · forge-fc · glacier-fc · granite-athletic
    │       mirage-sc · nebula-rangers · phoenix-rovers · tempest-united
    │
    ├── matrices/                ← resultatmatriser från turneringar (v6-rr-*)
    └── policies/                ← äldre policyversioner (v1–v4) och grafer
```

---

## Träningsmotor — snabbreferens

Bygg och kör från `training-engine/`:

```bash
cargo build --release

# Träna ett lag (3-stegs annealing, ~375k eval)
./target/release/training-engine --v6-team-train <lag> --quick

# Enstegstäning (bäst för finjustering)
./target/release/training-engine --single-stage <lag> <epoker> <matcher>

# Turneringstest (round-robin alla lag)
./target/release/training-engine --v6-tournament <matcher>

# Regenerera layout.svg
./target/release/training-engine --v6-team-svgs
```

Se `data/teams/COACH_RULES.md` för fullständig kommandoreferens och tränarregler.

---

## Coaching-system

Varje lag har tre filer som styr träningen:

| Fil | Syfte |
|-----|-------|
| `coach.md` | Tränarens persona — vem de är, hur de tänker, hur de pratar |
| `coaching.md` | Tränarjournal — vad som har gjorts, vad som fungerar, aktiv plan |
| `COACH_RULES.md` | Ligregler — inga direkta edits av baseline.json, krav på dokumentation |

Aktivera tränarrollen med `/football-coach <lagnamn>`.

---

## Frontend

Serveras lokalt med `http-server` (se `.claude/launch.json`):

```bash
npx http-server . -p 8765 --cors -c-1
```

Öppna `http://localhost:8765` i webbläsaren.

---

## Viktig konvention

- **`baseline.json` redigeras aldrig direkt** — all parameterförändring sker via träningskommandon eller `coach_nudge.py`
- **Tränarjournalen (`coaching.md`) hålls uppdaterad** efter varje träningssession
- **`layout.svg` regenereras** efter varje session med `--v6-team-svgs`
