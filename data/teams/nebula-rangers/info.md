# nebula-rangers

> Nebula Rangers — diffuse, exploratory

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 378/388/439 | 234/283/283 | 52/52/55 | 0/255/339 | 86/88/394 |
| MID-T | 503/586/586 | 160/322/488 | 66/82/91 | 64/229/302 | 0/96/236 |
| MID-B | 261/520/900 | 258/346/423 | 17/20/84 | 0/73/338 | 7/148/233 |
| DEF | 155/644/696 | 144/176/205 | 119/322/597 | 4/59/60 | 7/64/217 |
| GK | 53/102/215 | 3/3/42 | 364/374/374 | 23/309/384 | 264/264/353 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.132 | 0.79 | 1.74 | 0.02 | 1.95 | 1.39 |
| MID-T | 0.220 | 0.56 | 0.00 | 0.79 | 0.20 | 0.19 |
| MID-B | 0.054 | 0.86 | 1.96 | 0.77 | 1.44 | 1.81 |
| DEF | 0.090 | 0.57 | 1.74 | 0.17 | 0.15 | 0.23 |
| GK | 0.195 | 0.79 | 1.82 | 0.61 | 0.32 | 0.90 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 8 / 9

**Points**: 625 (W179 D88 L683)

**Goal-diff**: -3575

**Best vs**: granite-athletic

**Worst vs**: phoenix-rovers
