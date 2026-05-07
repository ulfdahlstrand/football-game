# Tränarregler — Alla Lag

Dessa regler gäller för alla tränare i ligan.

---

## Regler

1. **Ingen direkt filredigering** — tränare får INTE redigera `baseline.json` direkt (varken med texteditor, Python-script eller annan kod). Enda tillåtna sätten att påverka parametrar är träningskommandon eller nudge-verktyget.

2. **Turneringstest krävs** — kör `--v6-tournament 500` efter varje träningssession och notera resultatet i `coaching.md`.

3. **Uppdatera layout.svg** — kör `--v6-team-svgs` efter varje session för att hålla visualiseringen aktuell.

4. **Minimum 200 games per epoch** — kör aldrig träning med färre matcher än 200 per epoch.

5. **Tränarjournal** — håll `coaching.md` i lagets mapp uppdaterad med träningshistorik, resultat och lärdomar.

---

## Träningskommandon (referens)

| Kommando | Beskrivning |
|---|---|
| `--single-stage-slot <lag> <slot> <ep> <g>` | Tränar en specifik spelares slot (0=fwd, 1=mid, 2=mid, 3=def, 4=gk) |
| `--v6-team-train <lag> --quick` | 3-stegs annealing, ~375k eval |
| `--v6-team-train <lag> --short` | 3-stegs annealing, ~3M eval |
| `--single-stage <lag> <ep> <g>` | Enstegs utforskning, antal epoker × matcher |
| `--score-probe <lag_a> <lag_b> <g>` | Kör testmatcher mellan två lag |
| `--v6-tournament <g>` | Round-robin alla lag, ger tabellplacering |
| `--v6-team-svgs` | Regenererar layout.svg för alla lag |

---

## Nudge-verktyget

Tillåter kontrollerade manuella parameterknuffar när tillräckligt med träning skett.

**Gränser:**
- 1 nudge per 100 000 träningsevalueringar sedan föregående nudge
- Max Δ = ±0.05 per nudge
- Parametervärdet klampas automatiskt till giltigt intervall

```bash
# Kontrollera status
python3 coach_nudge.py status <lag>

# Registrera genomförd träning
python3 coach_nudge.py record-training <lag> <antal_evalueringar>

# Applicera en nudge (kräver bekräftelse)
python3 coach_nudge.py nudge <lag> <slot> <param> <delta>
```

---

## Poängsystem

- Vinst: 3 poäng
- Oavgjort: 1 poäng
- Förlust: 0 poäng
- Tabellen sorteras på poäng, sedan målskillnad (GD)
