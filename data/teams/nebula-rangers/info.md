# nebula-rangers

> Nebula Rangers — diffuse, exploratory

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 716/716/750 | 0/181/474 | 75/75/85 | 170/262/301 | 128/173/223 |
| MID-T | 293/293/336 | 198/297/297 | 0/9/27 | 81/108/400 | 28/205/313 |
| MID-B | 65/406/883 | 244/318/486 | 8/8/53 | 32/129/376 | 9/112/246 |
| DEF | 459/627/702 | 83/188/211 | 0/336/473 | 119/120/219 | 0/138/361 |
| GK | 136/205/373 | 0/73/98 | 469/550/589 | 98/218/391 | 16/139/249 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.211 | 0.77 | 1.44 | 0.39 | 1.71 | 1.78 |
| MID-T | 0.154 | 0.68 | 0.58 | 1.00 | 0.22 | 0.28 |
| MID-B | 0.117 | 0.81 | 1.88 | 0.82 | 1.70 | 1.44 |
| DEF | 0.069 | 0.84 | 1.81 | 0.09 | 0.64 | 0.30 |
| GK | 0.176 | 0.69 | 1.06 | 0.42 | 0.82 | 0.75 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 5 / 9

**Points**: 1280 (W412 D44 L344)

**Goal-diff**: +353

**Best vs**: eclipse-town

**Worst vs**: mirage-sc
