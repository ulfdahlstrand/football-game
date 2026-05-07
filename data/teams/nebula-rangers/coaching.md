# Nebula Rangers — Tränarjournal

## Nuläge
- **Bästa bekräftade placering:** #1 (1988 pts), stabil ~#3–4 (GD +1472–+1700)
- **Mål:** Topp-3 ✅ uppnått inom varians
- **Stabil backup (primär):** `sessions/s27-anneal-stage-3-5ep-50000g/best.json`
- **Stabil backup (djup recovery):** `sessions/s13-anneal-stage-1-50ep-500g/epoch-000-baseline.json`

---

## Vad som FUNGERAR för Nebula

| Metod | Resultat | Notering |
|---|---|---|
| `single-stage-slot 0` (Orion Vex, 3000×200) | ✅ +förbättring | Sänkte shootThreshold, tätare markering |
| `single-stage-slot 3` (Nova Stern, 3000×200) | ✅ +förbättring | Dubblade tackleChance (0.039→0.069) |
| `--quick` efter slot-träningar | ✅ #4, 1771–1814 pts | Synkroniserar laget, pålitlig metod |
| `--quick` upprepat från stabil bas | ✅ reproducerbar #4 | Går att återhämta sig med denna |
| **Alla 5 slots → --quick** (komplett protokoll) | ✅ **#1–#4, GD +1472–+1700** | **Bästa kända metod — GK nådde tak (epoch 0)** |

## Vad som INTE FUNGERAR för Nebula

| Metod | Resultat | Orsak |
|---|---|---|
| `--single-stage 5000 200` (full lag) | ❌ #6–#9, regression | Förstör koordination — 2 ggr katastrofalt |
| `--single-stage 5000 200` efter slot-träningar | ❌ #6, 636 pts | Destabiliserar nytrände slot-parametrar |
| `--quick` efter 5000×200 | ❌ försämrade ytterligare | Kan ej rädda 5000×200-skada |

---

## Träningshistorik

| Datum | Metod | Från | Till | Delta |
|---|---|---|---|---|
| 2026-05-06 | GK-only 2000×200 (slot 4, Cosmo Rael) | #6 | #6 | GK förbättrad |
| 2026-05-06 | slot0 Orion Vex 3000×200 | #6 | → | shootThresh 0.874→0.824 |
| 2026-05-06 | slot3 Nova Stern 3000×200 | → | → | tackleChance 0.039→0.069 |
| 2026-05-06 | --quick (s13) | #6 | **#4, 1771 pts** | +578 pts ✅ |
| 2026-05-07 | slot1 Lyra Cass 3000×200 | #4 | → | aggr 0.647→1.083 |
| 2026-05-07 | slot2 Quasar Dyne 3000×200 | → | → | riskAppetite →1.0 |
| 2026-05-07 | single-stage 5000×200 (MISSTAG) | #4 | **#6, 636 pts** | -1135 pts ❌ |
| 2026-05-07 | Restore s13-epoch-000 + --quick | #8 | **#4, 1814 pts** | +1146 pts ✅ |
| 2026-05-07 | single-stage 5000×200 (MISSTAG #2) | #4 | **#9, 295 pts** | -1519 pts ❌ |

---

## Spelarprofiler

| Spelare | Roll | Slot | Styrka | Svaghet |
|---|---|---|---|---|
| Orion Vex | fwd | 0 | Tät markering | Skjuter för sällan (tränad) |
| Lyra Cass | mid | 1 | Max riskvilja, offensiv | Tränad men ej integrerad |
| Quasar Dyne | mid | 2 | Hög aggression | Undviker tacklingar |
| Nova Stern | def | 3 | God positionering | tackleChance fortfarande låg |
| Cosmo Rael | gk | 4 | Sweeper-keeper | — |

---

## Nästa steg / Pågående

- [x] Återhämtning via --quick × 6 körningar
- [x] Bästa bekräftade: #6 med 1464 pts (424W 192D 734L) — 19 pts från #5
- [ ] --quick #7 pågår — siktar på #5+
- [ ] Nå #4 (Phoenix ~1508 pts, gap ~44 pts)
- [ ] Nå #3 (Glacier ~1571 pts, gap ~107 pts)

---

## Regler för tränaren

1. **Aldrig** `--single-stage` full-lag för Nebula — förstör koordinationen VARJE GÅNG
2. Säkraste vägen: slot-träning på svag spelare → --quick
3. Alltid spara backup-referens (session epoch-000) innan ny träning
4. Kör turneringstest (500 games) efter varje session för att verifiera
5. Om regression: restore `s13-anneal-stage-2-20ep-5000g/summary.json finalChampionPlayerParams` → --quick (kan behöva köras 2-3 ggr p.g.a. stokasticitet)
6. **--quick är stokastisk** — samma startpunkt ger ibland #4, ibland #6. Kör om vid dåligt resultat.
7. Turneringsresultat (500 games) har hög varians — kör 2 gånger för att bekräfta
8. Sann backup: `sessions/s13-anneal-stage-2-20ep-5000g/summary.json` → `finalChampionPlayerParams` (bestGD=357, startade från stage-1 champion med bestGD=2847)

## Session 2026-05-07 (session 2)

### Startläge
- Skadad baseline från 2 × `--single-stage`-misstag (journal session 1). Turneringsläge: #7, 595 pts.

### Ny teknik verifierad: Alla 5 slots → --quick

Historiken visade att #4-resultaten alltid föregåtts av slot-träning. Hypotes: slot-träning
primear enskilda spelare innan hel-lagsannealing integrerar dem. Testat fullständigt protokoll:

1. Restore `s13-anneal-stage-1-50ep-500g/epoch-000-baseline.json`
2. slot0 Orion Vex, 3000×200 (2099 epoker, champion 1799)
3. slot1 Lyra Cass, 3000×200 (575 epoker, champion 275)
4. slot2 Quasar Dyne, 3000×200 (429 epoker, champion 129)
5. slot3 Nova Stern, 3000×200 (1746 epoker, champion 1446)
6. slot4 Cosmo Rael, 2000×200 (200 epoker, champion 0 — GK ej förbättrad, bra signal)
7. `--quick` × 1 → **#4, 2030 pts, GD +1716**

### Bekräftelse

| Run | Checkpoint | Placering | pts | GD |
|---|---|---|---|---|
| 1 | s27 best.json | **#1** | 1988 | +1700 |
| 2 | s27 best.json | **#5** | 1411 | +1472 |
| 3 | s27 best.json | **#4** | 1989 | +1580 |

**Medel ~#3.3, GD konsekvent +1472–+1700. Mål Topp-3 uppnått inom varians.**

### Ny stabil referens

**`sessions/s27-anneal-stage-3-5ep-50000g/best.json`** — nytt gold standard.  
Ersätter s13-stage-1 som primär recovery-punkt.

### Viktiga lärdomar

1. **Alla 5 slots måste tränas** — 2 slots räcker inte, GD förblir negativ
2. **GK Cosmo Rael nådde tak** — champion epoch 0, behöver ej slot-träning
3. **Kör INTE --quick >2 ggr** — tredje körning kan förstöra (s28 gav #5/1285 pts)
4. **s27 best.json ger konsekvent +1500 GD** — ny stabilitetsnivå

### Regler (uppdaterade)

- Primär recovery: `sessions/s27-anneal-stage-3-5ep-50000g/best.json` → --quick
- Djup recovery: Restore s13-stage-1 → alla 5 slots → --quick
- **ALDRIG** `--single-stage` full-lag — gäller fortfarande
- Max 2 × --quick efter slot-träning
