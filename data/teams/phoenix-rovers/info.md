# phoenix-rovers

> Phoenix Rovers — energetic, comeback-prone

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 552/589/693 | 246/246/268 | 0/186/223 | 68/88/88 | 77/142/321 |
| MID-T | 54/187/228 | 233/308/373 | 0/368/612 | 72/83/88 | 0/26/44 |
| MID-B | 117/170/239 | 0/61/255 | 0/0/3 | 3/89/215 | 11/79/279 |
| DEF | 395/472/472 | 389/411/480 | 13/13/22 | 33/343/377 | 44/116/215 |
| GK | 0/126/282 | 55/55/210 | 38/143/173 | 173/263/301 | 79/258/285 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.208 | 0.55 | 1.66 | 0.16 | 0.34 | 0.11 |
| MID-T | 0.106 | 0.60 | 1.17 | 0.86 | 1.99 | 0.14 |
| MID-B | 0.220 | 0.76 | 1.64 | 0.64 | 0.37 | 1.17 |
| DEF | 0.219 | 0.72 | 1.89 | 0.00 | 0.63 | 1.10 |
| GK | 0.035 | 0.86 | 1.50 | 0.77 | 0.00 | 1.08 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 3 / 9

**Points**: 2051 (W652 D95 L203)

**Goal-diff**: +2080

**Best vs**: mirage-sc

**Worst vs**: glacier-fc
