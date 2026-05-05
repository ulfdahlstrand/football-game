# eclipse-town

> Eclipse Town — dark horse, counter-attack

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 444/533/697 | 98/312/358 | 52/183/382 | 143/165/392 | 38/38/211 |
| MID-T | 32/272/849 | 0/178/256 | 0/19/19 | 182/354/373 | 30/125/143 |
| MID-B | 131/131/337 | 225/225/408 | 0/10/44 | 0/33/95 | 36/98/159 |
| DEF | 493/493/500 | 26/72/145 | 151/380/517 | 85/87/87 | 75/184/400 |
| GK | 370/501/661 | 205/207/207 | 138/175/363 | 116/270/288 | 294/376/400 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.051 | 0.55 | 1.61 | 0.88 | 1.41 | 0.03 |
| MID-T | 0.220 | 0.68 | 0.26 | 0.34 | 1.83 | 0.19 |
| MID-B | 0.122 | 0.89 | 1.55 | 0.54 | 1.61 | 0.00 |
| DEF | 0.157 | 0.70 | 1.47 | 0.33 | 0.62 | 2.00 |
| GK | 0.080 | 0.78 | 0.53 | 0.94 | 1.30 | 0.64 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 7 / 9

**Points**: 659 (W193 D80 L552)

**Goal-diff**: -3217

**Best vs**: nebula-rangers

**Worst vs**: aurora-fc
