# aurora-fc

> Aurora FC — graceful and fluid

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 615/810/810 | 302/346/397 | 0/206/420 | 166/196/291 | 44/109/315 |
| MID-T | 0/421/581 | 160/287/391 | 20/20/193 | 56/61/294 | 12/43/247 |
| MID-B | 166/222/223 | 36/151/418 | 0/2/2 | 21/289/317 | 57/59/158 |
| DEF | 387/400/501 | 367/382/430 | 0/18/18 | 67/156/192 | 0/184/242 |
| GK | 93/203/900 | 366/408/513 | 561/561/638 | 192/330/334 | 129/284/368 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.099 | 0.58 | 1.83 | 0.80 | 1.47 | 0.69 |
| MID-T | 0.220 | 0.77 | 2.00 | 0.08 | 0.42 | 1.63 |
| MID-B | 0.220 | 0.55 | 2.00 | 0.58 | 1.39 | 1.41 |
| DEF | 0.182 | 0.55 | 1.66 | 0.55 | 0.99 | 0.00 |
| GK | 0.100 | 0.63 | 0.46 | 0.91 | 1.10 | 0.50 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 5 / 9

**Points**: 1766 (W579 D29 L192)

**Goal-diff**: +2762

**Best vs**: forge-fc

**Worst vs**: glacier-fc
