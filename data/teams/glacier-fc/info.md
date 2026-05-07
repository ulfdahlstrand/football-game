# glacier-fc

> Glacier FC — slow but inevitable

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 66/267/560 | 87/464/503 | 21/21/128 | 29/210/352 | 13/57/389 |
| MID-T | 137/243/243 | 116/192/283 | 41/213/520 | 71/104/133 | 0/7/205 |
| MID-B | 775/784/784 | 165/174/218 | 90/412/685 | 8/61/95 | 31/31/110 |
| DEF | 480/790/816 | 92/341/422 | 0/30/30 | 80/98/99 | 48/84/85 |
| GK | 88/370/487 | 170/207/223 | 14/141/339 | 62/223/349 | 154/269/337 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.189 | 0.55 | 1.53 | 0.88 | 0.40 | 0.32 |
| MID-T | 0.214 | 0.64 | 1.38 | 0.84 | 0.96 | 0.30 |
| MID-B | 0.087 | 0.55 | 1.10 | 0.08 | 0.75 | 0.55 |
| DEF | 0.130 | 0.56 | 1.11 | 0.54 | 0.94 | 0.06 |
| GK | 0.110 | 0.78 | 0.74 | 0.40 | 0.86 | 2.00 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 4 / 9

**Points**: 1815 (W590 D45 L165)

**Goal-diff**: +1993

**Best vs**: forge-fc

**Worst vs**: nebula-rangers
