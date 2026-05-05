#!/usr/bin/env python3
"""
compute-team-stats.py
Härleder formation och statistik från tränad baseline.json för varje lag
och skriver resultaten till respektive roster.json.

Kör:  python3 scripts/compute-team-stats.py
      python3 scripts/compute-team-stats.py --dry-run   (visar utan att skriva)
"""

import json, os, sys
from collections import Counter

FW       = 880          # spelplanens bredd (pixlar)
TEAMS_DIR = os.path.join(os.path.dirname(__file__), '..', 'data', 'teams')
DRY_RUN  = '--dry-run' in sys.argv

# ── Trösklar för formations-zoner ───────────────────────────────────────────
ZONE_DEF = 35   # < 35% av FW → DEF
ZONE_FWD = 65   # > 65% av FW → FWD
# däremellan → MID


def load_teams():
    teams = {}
    for slug in sorted(os.listdir(TEAMS_DIR)):
        bl = os.path.join(TEAMS_DIR, slug, 'baseline.json')
        ro = os.path.join(TEAMS_DIR, slug, 'roster.json')
        if os.path.exists(bl) and os.path.exists(ro):
            with open(bl, encoding='utf-8') as f:
                baseline = json.load(f)
            with open(ro, encoding='utf-8') as f:
                roster = json.load(f)
            teams[slug] = {'baseline': baseline, 'roster': roster, 'path': ro}
    return teams


def outfield_params(player_params):
    """Slots 0-3 med spatial + decisions (slot 4 = GK)."""
    return [
        p for i, p in enumerate(player_params)
        if i < 4 and p.get('spatial') and p.get('decisions')
    ]


def player_zone(p):
    pct = p['spatial']['ownGoal']['preferred'] / FW * 100
    return 'DEF' if pct < ZONE_DEF else 'MID' if pct < ZONE_FWD else 'FWD'


def compute_formation(player_params):
    outfield = outfield_params(player_params)
    counts = Counter(player_zone(p) for p in outfield)
    return '-'.join(
        str(counts[z]) for z in ['DEF', 'MID', 'FWD'] if counts.get(z, 0) > 0
    )


def raw_stats(player_params):
    """Beräkna råvärden (ej normaliserade) för ett lag."""
    outfield = outfield_params(player_params)
    zones    = [player_zone(p) for p in outfield]

    def avg(seq):
        return sum(seq) / len(seq) if seq else 0

    def_players = [p for p, z in zip(outfield, zones) if z == 'DEF']
    fwd_players = [p for p, z in zip(outfield, zones) if z == 'FWD']

    # Attack: låg shootProgressThreshold på FWD-spelare → anfallsbenägen
    fwd_thresh = [p['decisions']['shootProgressThreshold']
                  for p in (fwd_players or outfield)]
    attack = 1.0 - avg(fwd_thresh)

    # Försvar: hög tackleChance på DEF-spelare → defensiv styrka
    def_tackle = [p['decisions']['tackleChance']
                  for p in (def_players or outfield)]
    defense = avg(def_tackle)

    # Aggressivitet: decisions.aggression (tränat 0-2+)
    aggression = avg([p['decisions']['aggression'] for p in outfield])

    # Press: lägre markDistance → tightare marking → högre press-värde
    mark = avg([p['decisions']['markDistance'] for p in outfield])
    pressing = 1.0 / mark if mark > 0 else 0

    # Risktagande: riskAppetite (0–1+)
    risk = avg([p['decisions']['riskAppetite'] for p in outfield])

    # Direktspel: kvot offensiva passriktningar vs defensiva
    off_dir = avg([p['decisions']['passDirOffensive'] for p in outfield])
    def_dir = avg([p['decisions']['passDirDefensive'] for p in outfield])
    direct_play = off_dir / (off_dir + def_dir) if (off_dir + def_dir) > 0 else 0.5

    return {
        'attack':     attack,
        'defense':    defense,
        'aggression': aggression,
        'pressing':   pressing,
        'risk':       risk,
        'directPlay': direct_play,
    }


def normalize_all(raw_by_team):
    """Min-max-normalisera varje dimension till 40-99 (undviker 0/100)."""
    keys = list(next(iter(raw_by_team.values())).keys())
    lo = {k: min(v[k] for v in raw_by_team.values()) for k in keys}
    hi = {k: max(v[k] for v in raw_by_team.values()) for k in keys}

    result = {}
    for slug, raw in raw_by_team.items():
        result[slug] = {}
        for k in keys:
            span = hi[k] - lo[k]
            if span < 1e-9:
                result[slug][k] = 70  # alla lika → sätt mittenvärde
            else:
                result[slug][k] = round(40 + (raw[k] - lo[k]) / span * 59)
    return result


def main():
    teams = load_teams()
    if not teams:
        print("Inga lag med baseline.json hittades.")
        return

    # Beräkna råvärden
    raw = {slug: raw_stats(d['baseline']['playerParams']) for slug, d in teams.items()}
    formations = {slug: compute_formation(d['baseline']['playerParams']) for slug, d in teams.items()}
    normalized = normalize_all(raw)

    # Skriv ut tabell
    print(f"\n{'LAG':<22} {'FORMAT':<8} {'ATK':>4} {'DEF':>4} {'AGG':>4} {'PRESS':>6} {'RISK':>5} {'DIREKT':>7}")
    print('─' * 70)
    for slug in teams:
        n = normalized[slug]
        print(
            f"{slug:<22} {formations[slug]:<8}"
            f" {n['attack']:>4} {n['defense']:>4} {n['aggression']:>4}"
            f" {n['pressing']:>6} {n['risk']:>5} {n['directPlay']:>7}"
        )

    if DRY_RUN:
        print('\n[dry-run] Inga filer skrevs.')
        return

    # Uppdatera roster.json
    for slug, d in teams.items():
        roster = d['roster']
        n = normalized[slug]
        roster['formation'] = formations[slug]
        roster['rating'] = {
            'attack':     n['attack'],
            'defense':    n['defense'],
            'aggression': n['aggression'],
            'pressing':   n['pressing'],
            'risk':       n['risk'],
            'directPlay': n['directPlay'],
        }
        with open(d['path'], 'w', encoding='utf-8') as f:
            json.dump(roster, f, ensure_ascii=False, indent=2)
        print(f'✓  {slug}  →  {formations[slug]}')

    print('\nKlar! roster.json uppdaterad för alla lag.')


if __name__ == '__main__':
    main()
