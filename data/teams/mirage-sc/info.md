# mirage-sc

> Mirage SC — deceptive, unpredictable

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 536/574/687 | 158/167/479 | 10/28/60 | 57/208/257 | 6/274/274 |
| MID-T | 79/85/354 | 134/254/350 | 1/1/5 | 12/192/388 | 41/83/285 |
| MID-B | 359/374/611 | 347/347/470 | 9/9/28 | 93/101/355 | 14/64/379 |
| DEF | 176/364/849 | 0/307/349 | 11/12/140 | 91/177/400 | 0/54/117 |
| GK | 259/547/574 | 126/138/228 | 92/121/143 | 94/168/266 | 165/213/213 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.174 | 0.55 | 1.72 | 0.76 | 0.83 | 0.98 |
| MID-T | 0.220 | 0.56 | 1.63 | 0.48 | 0.07 | 0.62 |
| MID-B | 0.220 | 0.55 | 1.35 | 0.13 | 0.78 | 1.23 |
| DEF | 0.087 | 0.60 | 0.38 | 0.55 | 0.39 | 0.00 |
| GK | 0.170 | 0.58 | 0.99 | 0.57 | 0.21 | 1.62 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 1 / 9

**Points**: 2156 (W665 D161 L299)

**Goal-diff**: +1343

**Best vs**: glacier-fc

**Worst vs**: aurora-fc
