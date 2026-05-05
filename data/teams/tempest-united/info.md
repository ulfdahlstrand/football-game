# tempest-united

> Tempest United — stormy, chaotic press

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 380/616/800 | 46/58/124 | 0/2/2 | 195/278/306 | 44/199/203 |
| MID-T | 115/252/347 | 101/335/346 | 0/114/619 | 25/68/68 | 0/62/343 |
| MID-B | 2/441/663 | 337/408/501 | 0/3/39 | 0/2/75 | 0/28/48 |
| DEF | 411/411/452 | 62/277/277 | 26/26/43 | 13/58/58 | 17/80/80 |
| GK | 190/537/827 | 332/383/383 | 33/387/683 | 167/228/276 | 39/138/206 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.220 | 0.69 | 1.86 | 0.54 | 0.76 | 1.00 |
| MID-T | 0.090 | 0.68 | 1.61 | 0.36 | 2.00 | 1.75 |
| MID-B | 0.220 | 0.55 | 1.06 | 0.80 | 1.20 | 1.38 |
| DEF | 0.080 | 0.55 | 1.93 | 0.03 | 1.91 | 0.06 |
| GK | 0.032 | 0.79 | 1.64 | 0.57 | 1.49 | 0.69 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 1 / 9

**Points**: 2898 (W898 D204 L648)

**Goal-diff**: +884

**Best vs**: phoenix-rovers

**Worst vs**: glacier-fc
