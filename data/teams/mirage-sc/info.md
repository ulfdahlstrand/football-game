# mirage-sc

> Mirage SC — deceptive, unpredictable

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 540/540/585 | 91/179/473 | 0/13/35 | 2/229/271 | 0/200/253 |
| MID-T | 89/153/153 | 152/174/339 | 0/6/6 | 59/190/376 | 90/124/284 |
| MID-B | 234/329/453 | 141/413/439 | 3/4/39 | 54/64/341 | 19/94/270 |
| DEF | 274/375/708 | 53/309/365 | 31/43/108 | 98/167/400 | 71/84/187 |
| GK | 413/439/482 | 120/172/172 | 111/116/116 | 114/153/194 | 39/204/260 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.164 | 0.55 | 1.34 | 0.56 | 0.91 | 0.80 |
| MID-T | 0.156 | 0.66 | 1.66 | 0.40 | 0.04 | 0.50 |
| MID-B | 0.131 | 0.67 | 1.41 | 0.24 | 0.77 | 0.49 |
| DEF | 0.013 | 0.55 | 0.31 | 0.83 | 0.26 | 0.02 |
| GK | 0.170 | 0.58 | 0.99 | 0.57 | 0.21 | 1.62 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

_(filled in after `--v6-tournament` run)_
