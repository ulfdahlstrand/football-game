# phoenix-rovers

> Phoenix Rovers — energetic, comeback-prone

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 516/627/627 | 246/246/284 | 0/204/275 | 68/130/130 | 110/118/297 |
| MID-T | 0/187/228 | 268/295/373 | 26/335/612 | 68/68/72 | 22/24/63 |
| MID-B | 156/170/217 | 0/77/252 | 0/0/3 | 3/132/215 | 11/79/287 |
| DEF | 432/583/667 | 389/389/480 | 13/21/21 | 47/327/377 | 33/113/242 |
| GK | 0/99/331 | 40/40/219 | 0/143/260 | 152/263/330 | 61/258/312 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.199 | 0.55 | 1.59 | 0.14 | 0.37 | 0.11 |
| MID-T | 0.106 | 0.58 | 1.17 | 0.86 | 1.86 | 0.21 |
| MID-B | 0.220 | 0.76 | 1.78 | 0.61 | 0.37 | 1.17 |
| DEF | 0.204 | 0.73 | 1.89 | 0.00 | 0.44 | 1.41 |
| GK | 0.035 | 0.86 | 1.50 | 0.77 | 0.00 | 1.08 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 2 / 9

**Points**: 2644 (W830 D154 L516)

**Goal-diff**: +2083

**Best vs**: tempest-united

**Worst vs**: glacier-fc
