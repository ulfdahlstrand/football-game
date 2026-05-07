# granite-athletic

> Granite Athletic — solid and robust

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 392/392/426 | 226/302/334 | 14/39/82 | 74/110/113 | 1/66/292 |
| MID-T | 9/355/712 | 162/271/427 | 29/69/69 | 120/204/236 | 0/91/132 |
| MID-B | 183/183/190 | 231/254/381 | 0/0/0 | 8/270/376 | 25/102/191 |
| DEF | 0/269/549 | 0/64/394 | 11/49/61 | 0/164/400 | 17/81/193 |
| GK | 123/182/182 | 395/395/497 | 458/508/508 | 113/203/400 | 54/108/128 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.077 | 0.75 | 0.00 | 1.00 | 2.00 | 0.52 |
| MID-T | 0.201 | 0.59 | 1.65 | 0.49 | 0.58 | 0.52 |
| MID-B | 0.209 | 0.55 | 1.24 | 0.72 | 2.00 | 0.30 |
| DEF | 0.036 | 0.70 | 1.52 | 0.83 | 1.46 | 1.06 |
| GK | 0.220 | 0.60 | 0.82 | 0.11 | 1.55 | 0.56 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 1 / 9

**Points**: 2724 (W778 D390 L857)

**Goal-diff**: -608

**Best vs**: forge-fc

**Worst vs**: aurora-fc
