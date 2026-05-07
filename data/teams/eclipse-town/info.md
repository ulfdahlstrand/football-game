# eclipse-town

> Eclipse Town — dark horse, counter-attack

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 544/617/638 | 283/283/286 | 14/485/530 | 90/306/348 | 61/226/353 |
| MID-T | 0/197/691 | 88/253/389 | 64/68/68 | 180/276/325 | 0/26/28 |
| MID-B | 77/140/175 | 134/406/435 | 0/1/3 | 112/112/159 | 44/156/381 |
| DEF | 188/188/193 | 103/110/235 | 72/328/563 | 111/112/112 | 106/331/341 |
| GK | 232/392/572 | 87/189/240 | 0/28/591 | 164/164/207 | 275/372/387 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.019 | 0.74 | 1.06 | 0.95 | 0.62 | 0.00 |
| MID-T | 0.107 | 0.81 | 0.98 | 0.87 | 0.52 | 0.07 |
| MID-B | 0.121 | 0.70 | 1.95 | 0.16 | 1.70 | 0.28 |
| DEF | 0.191 | 0.58 | 0.98 | 0.02 | 1.43 | 1.68 |
| GK | 0.080 | 0.78 | 0.53 | 0.94 | 1.30 | 0.64 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 7 / 9

**Points**: 1252 (W383 D103 L639)

**Goal-diff**: -1819

**Best vs**: nebula-rangers

**Worst vs**: phoenix-rovers
