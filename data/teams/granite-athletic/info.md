# granite-athletic

> Granite Athletic — solid and robust

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 339/379/379 | 201/369/385 | 0/42/183 | 55/55/91 | 4/32/233 |
| MID-T | 68/438/438 | 41/316/405 | 0/0/21 | 106/163/225 | 47/70/88 |
| MID-B | 183/183/187 | 185/234/332 | 0/0/0 | 42/288/396 | 19/129/179 |
| DEF | 63/63/266 | 44/89/471 | 53/113/170 | 52/100/331 | 26/96/115 |
| GK | 1/94/94 | 361/361/520 | 516/520/520 | 88/193/400 | 47/115/123 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.119 | 0.65 | 0.33 | 0.85 | 1.80 | 0.12 |
| MID-T | 0.220 | 0.55 | 1.77 | 0.84 | 0.71 | 0.36 |
| MID-B | 0.181 | 0.55 | 1.01 | 0.63 | 1.78 | 0.66 |
| DEF | 0.061 | 0.76 | 1.61 | 0.77 | 1.87 | 1.76 |
| GK | 0.220 | 0.60 | 0.82 | 0.11 | 1.55 | 0.56 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

_(filled in after `--v6-tournament` run)_
