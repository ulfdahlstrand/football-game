# tempest-united

> Tempest United — stormy, chaotic press

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 679/779/779 | 156/208/409 | 102/133/133 | 173/271/358 | 78/122/180 |
| MID-T | 91/170/362 | 26/251/456 | 0/30/413 | 11/66/126 | 16/28/308 |
| MID-B | 7/459/597 | 351/386/386 | 0/1/3 | 0/2/11 | 38/79/79 |
| DEF | 455/455/455 | 166/188/225 | 27/28/28 | 25/50/52 | 37/37/124 |
| GK | 138/254/525 | 285/380/416 | 17/441/441 | 101/206/309 | 20/127/191 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.102 | 0.79 | 1.13 | 0.52 | 0.46 | 1.72 |
| MID-T | 0.175 | 0.90 | 1.91 | 0.02 | 2.00 | 1.92 |
| MID-B | 0.211 | 0.59 | 1.68 | 0.76 | 0.85 | 1.40 |
| DEF | 0.060 | 0.64 | 1.63 | 0.00 | 1.61 | 0.75 |
| GK | 0.032 | 0.79 | 1.64 | 0.57 | 1.49 | 0.69 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 1 / 9

**Points**: 2343 (W762 D57 L206)

**Goal-diff**: +3901

**Best vs**: mirage-sc

**Worst vs**: aurora-fc
