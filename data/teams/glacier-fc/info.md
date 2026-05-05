# glacier-fc

> Glacier FC — slow but inevitable

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 0/224/640 | 84/504/520 | 21/21/110 | 32/210/354 | 0/57/381 |
| MID-T | 180/226/280 | 58/281/292 | 21/229/520 | 88/88/115 | 3/32/221 |
| MID-B | 681/746/788 | 161/174/200 | 47/412/665 | 9/41/109 | 27/50/125 |
| DEF | 480/771/816 | 100/309/387 | 0/24/30 | 96/99/99 | 15/84/130 |
| GK | 172/356/441 | 156/156/161 | 102/145/339 | 68/204/377 | 220/262/346 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.174 | 0.55 | 1.45 | 1.00 | 0.40 | 0.46 |
| MID-T | 0.220 | 0.64 | 1.38 | 0.90 | 0.80 | 0.00 |
| MID-B | 0.099 | 0.55 | 1.15 | 0.00 | 0.75 | 0.46 |
| DEF | 0.130 | 0.57 | 1.21 | 0.23 | 0.94 | 0.32 |
| GK | 0.110 | 0.78 | 0.74 | 0.40 | 0.86 | 2.00 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 5 / 9

**Points**: 1747 (W560 D67 L223)

**Goal-diff**: +2106

**Best vs**: eclipse-town

**Worst vs**: mirage-sc
