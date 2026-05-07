# forge-fc

> forge-fc

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 463/533/676 | 0/257/426 | 43/43/386 | 1/42/96 | 0/43/263 |
| MID-T | 67/442/724 | 104/249/401 | 29/53/118 | 120/128/128 | 0/129/173 |
| MID-B | 0/158/749 | 179/417/463 | 0/0/83 | 8/51/332 | 0/173/250 |
| DEF | 193/197/399 | 69/261/520 | 15/19/191 | 39/109/172 | 8/8/39 |
| GK | 51/86/86 | 72/397/397 | 174/285/586 | 0/107/124 | 60/158/319 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.041 | 0.58 | 1.88 | 0.63 | 0.73 | 0.75 |
| MID-T | 0.220 | 0.80 | 1.70 | 0.86 | 0.72 | 1.56 |
| MID-B | 0.125 | 0.87 | 1.11 | 0.15 | 1.40 | 0.44 |
| DEF | 0.220 | 0.75 | 1.14 | 0.61 | 0.13 | 0.38 |
| GK | 0.080 | 0.76 | 1.00 | 0.50 | 1.00 | 1.00 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 6 / 9

**Points**: 1470 (W411 D237 L802)

**Goal-diff**: -1376

**Best vs**: tempest-united

**Worst vs**: glacier-fc
