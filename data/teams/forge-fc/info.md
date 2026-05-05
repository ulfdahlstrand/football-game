# forge-fc

> forge-fc

Trained from clustered start (all field players at centre of own half) via
adaptive multi-stage anneal. Spatial preferences emerged organically from
mutation + selection — no positional logic was hand-coded.

## Spatial preferences (min / preferred / max)

| Slot | own_goal | side | ball | teammate | opponent |
|------|----------|------|------|----------|----------|
| FWD | 150/380/700 | 60/260/460 | 20/20/280 | 40/220/220 | 30/30/260 |
| MID-T | 100/290/500 | 40/280/280 | 20/20/280 | 40/100/220 | 20/85/220 |
| MID-B | 100/290/500 | 240/480/480 | 20/20/280 | 40/40/220 | 20/20/220 |
| DEF | 40/320/320 | 60/260/460 | 30/30/320 | 50/50/220 | 15/15/150 |
| GK | 0/110/241 | 112/318/378 | 177/269/683 | 7/156/197 | 9/230/292 |

## Decision parameters

| Slot | tackle | shoot_thr | aggr | risk | passDirOff | passDirDef |
|------|--------|-----------|------|------|------------|------------|
| FWD | 0.080 | 0.64 | 2.00 | 0.50 | 1.00 | 1.00 |
| MID-T | 0.168 | 0.76 | 1.00 | 0.50 | 1.00 | 1.00 |
| MID-B | 0.080 | 0.90 | 1.00 | 0.00 | 1.00 | 0.00 |
| DEF | 0.220 | 0.76 | 1.00 | 0.50 | 1.00 | 1.00 |
| GK | 0.080 | 0.76 | 1.00 | 0.50 | 1.00 | 1.00 |

## Inferred strategy

_(filled in after tournament analysis — see matrix in `data/matrices/`)_

## Tournament

**Rank**: 9 / 9

**Points**: 569 (W169 D62 L569)

**Goal-diff**: -2093

**Best vs**: nebula-rangers

**Worst vs**: aurora-fc
