// ── Flashig lagväljare + förbättrad LineupScreen ─────────────────────────────
const { useState: useStateTS, useEffect: useEffectTS, useRef: useRefTS } = React;

// Beräkna formation från spelare (exkl. GK), back→mitt→forward
// Rollnamnen i roster.json är hårdkodade och stämmer inte alltid —
// framöver ersätts detta med positioner härledda från träningsdata.
function computeFormation(players) {
  if (!players || players.length === 0) return null;
  const outfield = players.filter(p => p.role !== 'gk');
  const defs = outfield.filter(p => p.role === 'def').length;
  const mids = outfield.filter(p => p.role === 'mid').length;
  const fwds = outfield.filter(p => p.role === 'fwd').length;
  return [defs, mids, fwds].filter(n => n > 0).join('-');
}

// Laginformation — namn, färger, slug
const TEAM_ROSTER = [
  { slug: 'aurora-fc',        name: 'Aurora FC',        primary: '#2d8a4e', secondary: '#1a5c34', accent: '#4ecdc4' },
  { slug: 'eclipse-town',     name: 'Eclipse Town',     primary: '#3a2d6e', secondary: '#1e1440', accent: '#a78bfa' },
  { slug: 'forge-fc',         name: 'Forge FC',         primary: '#7c2d0e', secondary: '#431407', accent: '#fb923c' },
  { slug: 'glacier-fc',       name: 'Glacier FC',       primary: '#1e4d7a', secondary: '#0d2d4f', accent: '#7dd3fc' },
  { slug: 'granite-athletic', name: 'Granite Athletic', primary: '#374151', secondary: '#1f2937', accent: '#9ca3af' },
  { slug: 'mirage-sc',        name: 'Mirage SC',        primary: '#78520a', secondary: '#3d2a04', accent: '#fbbf24' },
  { slug: 'nebula-rangers',   name: 'Nebula Rangers',   primary: '#5b21b6', secondary: '#2d1060', accent: '#f472b6' },
  { slug: 'phoenix-rovers',   name: 'Phoenix Rovers',   primary: '#9a1c0a', secondary: '#5a0e04', accent: '#fcd34d' },
  { slug: 'tempest-united',   name: 'Tempest United',   primary: '#1a3a6e', secondary: '#0d1f40', accent: '#38bdf8' },
];

const ROLE_LABEL = { fwd: 'ANF', mid: 'MIT', def: 'BAC', gk: 'MÅL' };
const ROLE_COLOR = { fwd: '#ef4444', mid: '#3b82f6', def: '#22c55e', gk: '#f59e0b' };

// Hämta och cacha roster-data
const rosterCache = {};
function fetchRoster(slug) {
  if (rosterCache[slug]) return Promise.resolve(rosterCache[slug]);
  return fetch(`data/teams/${slug}/roster.json?t=${Date.now()}`, { cache: 'no-store' })
    .then(r => r.json())
    .then(d => { rosterCache[slug] = d; return d; })
    .catch(() => null);
}

// ── Lagkort (liten version för val) ──────────────────────────────────────────
function TeamCard({ team, selected, onSelect, side }) {
  const [hovered, setHovered] = useStateTS(false);
  const isActive = selected || hovered;
  return (
    <button
      onClick={onSelect}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        background: isActive
          ? `linear-gradient(135deg, ${team.primary}, ${team.secondary})`
          : 'rgba(255,255,255,0.04)',
        border: `2px solid ${isActive ? team.accent : 'rgba(255,255,255,0.12)'}`,
        borderRadius: 10,
        padding: '12px 14px',
        cursor: 'pointer',
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        transition: 'all 0.18s ease',
        transform: isActive ? 'scale(1.03)' : 'scale(1)',
        boxShadow: isActive ? `0 0 20px ${team.accent}44, 0 4px 12px rgba(0,0,0,0.4)` : 'none',
        width: '100%',
        textAlign: 'left',
      }}
    >
      <img
        src={`data/teams/${team.slug}/logo.svg`}
        style={{ width: 40, height: 40, objectFit: 'contain', flexShrink: 0,
          filter: isActive ? 'drop-shadow(0 0 6px ' + team.accent + '88)' : 'none' }}
        alt=""
      />
      <div style={{ minWidth: 0 }}>
        <div style={{
          fontFamily: 'ui-monospace, Menlo, monospace',
          fontSize: 12, fontWeight: 700, letterSpacing: '0.05em',
          color: isActive ? '#fff' : 'rgba(255,255,255,0.7)',
          whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis'
        }}>
          {team.name}
        </div>
        {selected && (
          <div style={{
            fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
            letterSpacing: '0.15em', color: team.accent, marginTop: 2
          }}>
            {side === 0 ? '◀ DITT LAG' : 'MOTSTÅNDARE ▶'}
          </div>
        )}
      </div>
    </button>
  );
}

// ── Spelarpresentation ────────────────────────────────────────────────────────
function PlayerRow({ player, team, compact }) {
  return (
    <div style={{
      display: 'flex', alignItems: 'flex-start', gap: 10,
      padding: compact ? '8px 0' : '10px 0',
      borderBottom: '1px solid rgba(255,255,255,0.07)'
    }}>
      <div style={{
        flexShrink: 0, width: compact ? 28 : 36, height: compact ? 28 : 36,
        background: ROLE_COLOR[player.role] + '22',
        border: `1.5px solid ${ROLE_COLOR[player.role]}`,
        borderRadius: 4,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontFamily: 'ui-monospace, Menlo, monospace',
        fontSize: compact ? 7 : 8, fontWeight: 700,
        color: ROLE_COLOR[player.role], letterSpacing: '0.05em'
      }}>
        {ROLE_LABEL[player.role]}
      </div>
      <div style={{ minWidth: 0 }}>
        <div style={{
          fontFamily: 'ui-monospace, Menlo, monospace',
          fontSize: compact ? 11 : 13, fontWeight: 700,
          color: '#f3f4f6', letterSpacing: '0.02em'
        }}>
          {player.name}
        </div>
        {!compact && (
          <div style={{
            fontFamily: 'Georgia, serif', fontSize: 12,
            color: 'rgba(255,255,255,0.55)', lineHeight: 1.45,
            fontStyle: 'italic', marginTop: 2, textWrap: 'pretty'
          }}>
            {player.description}
          </div>
        )}
      </div>
    </div>
  );
}

// ── Ratings-bar ──────────────────────────────────────────────────────────────
function RatingBar({ label, value, accent, flip }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8,
      flexDirection: flip ? 'row-reverse' : 'row', marginBottom: 5 }}>
      <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
        color: 'rgba(255,255,255,0.45)', letterSpacing: '0.1em', width: 56,
        textAlign: flip ? 'left' : 'right' }}>
        {label}
      </div>
      <div style={{ flex: 1, height: 5, background: 'rgba(255,255,255,0.08)', borderRadius: 3, overflow: 'hidden' }}>
        <div style={{ height: '100%', width: `${value}%`, borderRadius: 3,
          background: `linear-gradient(90deg, ${accent}88, ${accent})`,
          transition: 'width 0.6s cubic-bezier(0.34,1.2,0.64,1)' }} />
      </div>
      <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
        color: accent, fontWeight: 700, width: 24, textAlign: flip ? 'right' : 'left' }}>
        {value}
      </div>
    </div>
  );
}

// ── Lagpanel (logga + roster info) ────────────────────────────────────────────
function TeamPanel({ team, roster, side, label }) {
  const [tab, setTab] = useStateTS('info'); // info | roster

  if (!team) return (
    <div style={{
      flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center',
      color: 'rgba(255,255,255,0.2)', fontFamily: 'ui-monospace, Menlo, monospace',
      fontSize: 12, letterSpacing: '0.15em'
    }}>
      VÄLJ LAG {side === 0 ? '▶' : '◀'}
    </div>
  );

  return (
    <div style={{
      flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden',
      background: `linear-gradient(160deg, ${team.primary}33 0%, transparent 60%)`,
      borderRadius: 12, border: `1px solid ${team.accent}22`, padding: '16px 16px 14px'
    }}>
      {/* Header */}
      <div style={{
        display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12,
        flexDirection: side === 1 ? 'row-reverse' : 'row'
      }}>
        <div style={{ position: 'relative', flexShrink: 0 }}>
          <div style={{ position: 'absolute', inset: -8, borderRadius: '50%',
            background: `radial-gradient(circle, ${team.accent}33 0%, transparent 70%)` }} />
          <img src={`data/teams/${team.slug}/logo.svg`}
            style={{ width: 60, height: 60, objectFit: 'contain', position: 'relative',
              filter: `drop-shadow(0 0 10px ${team.accent}66)` }} alt="" />
        </div>
        <div style={{ textAlign: side === 1 ? 'right' : 'left', minWidth: 0 }}>
          <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 8,
            letterSpacing: '0.2em', color: team.accent, fontWeight: 700, marginBottom: 2 }}>
            {label}
          </div>
          <div style={{ fontFamily: 'Georgia, serif', fontSize: 18, fontWeight: 700,
            color: '#fff', lineHeight: 1.1, textWrap: 'balance' }}>
            {team.name}
          </div>
          {roster?.style && (
            <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
              color: 'rgba(255,255,255,0.5)', marginTop: 3 }}>
              {roster.styleIcon} {roster.style} · {roster.formation}
            </div>
          )}
        </div>
      </div>

      {/* Stjärnspelare */}
      {roster?.star && (
        <div style={{
          display: 'flex', alignItems: 'center', gap: 10, marginBottom: 10,
          padding: '8px 10px',
          background: `linear-gradient(90deg, ${team.accent}18, transparent)`,
          border: `1px solid ${team.accent}33`, borderRadius: 6,
          flexDirection: side === 1 ? 'row-reverse' : 'row'
        }}>
          <div style={{ width: 28, height: 28, borderRadius: '50%',
            background: `${team.accent}33`, border: `2px solid ${team.accent}`,
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 8,
            color: team.accent, fontWeight: 700, flexShrink: 0 }}>
            ★
          </div>
          <div style={{ textAlign: side === 1 ? 'right' : 'left' }}>
            <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 8,
              color: team.accent, letterSpacing: '0.15em' }}>STJÄRNSPELARE</div>
            <div style={{ fontFamily: 'Georgia, serif', fontSize: 13, fontWeight: 700, color: '#fff' }}>
              {roster.star.name}
              <span style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
                color: 'rgba(255,255,255,0.5)', marginLeft: 6 }}>{roster.star.trait}</span>
            </div>
          </div>
        </div>
      )}

      {/* Tab-knappar */}
      <div style={{ display: 'flex', borderBottom: '1px solid rgba(255,255,255,0.08)',
        marginBottom: 10, flexShrink: 0 }}>
        {[['info','INFO'],['roster','TRUPP']].map(([k,l]) => (
          <button key={k} onClick={() => setTab(k)} style={{
            background: 'transparent', border: 'none', cursor: 'pointer',
            padding: '5px 14px', fontFamily: 'ui-monospace, Menlo, monospace',
            fontSize: 9, fontWeight: 700, letterSpacing: '0.15em',
            color: tab === k ? team.accent : 'rgba(255,255,255,0.3)',
            borderBottom: `2px solid ${tab === k ? team.accent : 'transparent'}`,
            marginBottom: -1, transition: 'color 0.15s'
          }}>{l}</button>
        ))}
      </div>

      {/* Tab-innehåll */}
      <div style={{ flex: 1, overflow: 'auto', minHeight: 0 }}>
        {tab === 'info' && roster?.rating && (
          <div>
            {/* Ratings */}
            <div style={{ marginBottom: 12 }}>
              <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 8,
                color: 'rgba(255,255,255,0.35)', letterSpacing: '0.15em', marginBottom: 6 }}>RATINGS</div>
              <RatingBar label="ATTACK"   value={roster.rating.attack}     accent={team.accent} flip={side===1} />
              <RatingBar label="FÖRSVAR"  value={roster.rating.defense}    accent={team.accent} flip={side===1} />
              <RatingBar label="AGGRESS." value={roster.rating.aggression} accent={team.accent} flip={side===1} />
              <RatingBar label="PRESS"    value={roster.rating.pressing}   accent={team.accent} flip={side===1} />
              <RatingBar label="RISK"     value={roster.rating.risk}       accent={team.accent} flip={side===1} />
              <RatingBar label="DIREKT"   value={roster.rating.directPlay} accent={team.accent} flip={side===1} />
            </div>

            {/* Styrkor */}
            {roster.strengths && (
              <div style={{ marginBottom: 10 }}>
                <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 8,
                  color: 'rgba(255,255,255,0.35)', letterSpacing: '0.15em', marginBottom: 5 }}>STYRKOR</div>
                {roster.strengths.map((s,i) => (
                  <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 3,
                    flexDirection: side === 1 ? 'row-reverse' : 'row' }}>
                    <div style={{ width: 6, height: 6, borderRadius: '50%',
                      background: '#22c55e', flexShrink: 0 }} />
                    <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10,
                      color: 'rgba(255,255,255,0.75)' }}>{s}</div>
                  </div>
                ))}
              </div>
            )}

            {/* Svagheter */}
            {roster.weaknesses && (
              <div>
                <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 8,
                  color: 'rgba(255,255,255,0.35)', letterSpacing: '0.15em', marginBottom: 5 }}>SVAGHETER</div>
                {roster.weaknesses.map((w,i) => (
                  <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 3,
                    flexDirection: side === 1 ? 'row-reverse' : 'row' }}>
                    <div style={{ width: 6, height: 6, borderRadius: '50%',
                      background: '#ef4444', flexShrink: 0 }} />
                    <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10,
                      color: 'rgba(255,255,255,0.75)' }}>{w}</div>
                  </div>
                ))}
              </div>
            )}

            {/* Beskrivning */}
            {roster?.description && (
              <div style={{ marginTop: 12, fontFamily: 'Georgia, serif', fontSize: 11,
                color: 'rgba(255,255,255,0.5)', lineHeight: 1.55, fontStyle: 'italic',
                borderTop: '1px solid rgba(255,255,255,0.08)', paddingTop: 10 }}>
                "{roster.description}"
              </div>
            )}
          </div>
        )}

        {tab === 'info' && !roster?.rating && roster?.description && (
          <div style={{ fontFamily: 'Georgia, serif', fontSize: 13,
            color: 'rgba(255,255,255,0.65)', lineHeight: 1.55, fontStyle: 'italic' }}>
            {roster.description}
          </div>
        )}

        {tab === 'roster' && roster?.players && roster.players.map(p => (
          <PlayerRow key={p.id} player={p} team={team} compact />
        ))}

        {!roster && (
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center',
            height: 60, color: 'rgba(255,255,255,0.3)', fontSize: 11,
            fontFamily: 'ui-monospace, Menlo, monospace', letterSpacing: '0.1em' }}>
            LADDAR…
          </div>
        )}
      </div>
    </div>
  );
}

// ── VS-animation ──────────────────────────────────────────────────────────────
function VSBadge({ team0, team1 }) {
  return (
    <div style={{
      display: 'flex', flexDirection: 'column', alignItems: 'center',
      justifyContent: 'center', gap: 6, flexShrink: 0, width: 80
    }}>
      <div style={{
        fontFamily: 'Georgia, serif', fontSize: 42, fontWeight: 900,
        background: 'linear-gradient(180deg, #fff 0%, rgba(255,255,255,0.4) 100%)',
        WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent',
        lineHeight: 1, filter: 'drop-shadow(0 0 20px rgba(255,255,255,0.3))',
        letterSpacing: '-0.04em'
      }}>VS</div>
      <div style={{
        width: 2, height: 40,
        background: 'linear-gradient(180deg, transparent, rgba(255,255,255,0.3), transparent)'
      }} />
    </div>
  );
}

// ── Huvud-lagväljarskärm ──────────────────────────────────────────────────────
function TeamSelectScreen({ opponents, groupedOpponents, highestVersionIdx, onStart, onExit }) {
  const [team0idx, setTeam0idx] = useStateTS(0);
  const [team1idx, setTeam1idx] = useStateTS(1);
  const [roster0, setRoster0] = useStateTS(null);
  const [roster1, setRoster1] = useStateTS(null);
  const [oppIdx0, setOppIdx0] = useStateTS(highestVersionIdx);
  const [oppIdx1, setOppIdx1] = useStateTS(highestVersionIdx);
  const [phase, setPhase] = useStateTS('select');

  const team0 = TEAM_ROSTER[team0idx];
  const team1 = TEAM_ROSTER[team1idx];

  // Hitta policy-index för ett lag-slug, annars highestVersionIdx
  const policyIdxForSlug = (slug) => {
    const idx = opponents.findIndex(o => o.name === slug);
    return idx !== -1 ? idx : highestVersionIdx;
  };

  // Ladda rosters + auto-välj policy när lag ändras
  useEffectTS(() => {
    fetchRoster(TEAM_ROSTER[team0idx].slug).then(setRoster0);
    if (opponents.length) setOppIdx0(policyIdxForSlug(TEAM_ROSTER[team0idx].slug));
  }, [team0idx, opponents.length]);

  useEffectTS(() => {
    fetchRoster(TEAM_ROSTER[team1idx].slug).then(setRoster1);
    if (opponents.length) setOppIdx1(policyIdxForSlug(TEAM_ROSTER[team1idx].slug));
  }, [team1idx, opponents.length]);

  // Gradient bakgrund baserad på valda lag
  const bgGrad = `linear-gradient(135deg,
    ${team0.secondary}ee 0%,
    #0a0a0f 45%,
    ${team1.secondary}ee 100%)`;

  return (
    <div style={{
      position: 'absolute', inset: 0,
      background: bgGrad,
      display: 'flex', flexDirection: 'column',
      fontFamily: '-apple-system, "Helvetica Neue", Helvetica, Arial, sans-serif',
      overflow: 'hidden'
    }}>
      {/* Subtil planstruktur i bakgrunden */}
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.04, pointerEvents: 'none' }}>
        <rect x="10%" y="5%" width="80%" height="90%" fill="none" stroke="#fff" strokeWidth="1.5" />
        <line x1="50%" y1="5%" x2="50%" y2="95%" stroke="#fff" strokeWidth="1.5" />
        <circle cx="50%" cy="50%" r="12%" fill="none" stroke="#fff" strokeWidth="1.5" />
        <circle cx="50%" cy="50%" r="0.5%" fill="#fff" />
      </svg>

      {/* Header */}
      <div style={{
        padding: '18px 28px 0',
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        flexShrink: 0
      }}>
        <div>
          <div style={{
            fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
            letterSpacing: '0.25em', color: 'rgba(255,255,255,0.4)'
          }}>FOTBOLLS-RPG · MATCHSTART</div>
          <div style={{
            fontFamily: 'Georgia, serif', fontSize: 26, fontWeight: 700,
            color: '#fff', marginTop: 2
          }}>Välj Lag</div>
        </div>
        <button onClick={onExit} style={{
          background: 'rgba(255,255,255,0.08)', color: 'rgba(255,255,255,0.6)',
          border: '1px solid rgba(255,255,255,0.15)', borderRadius: 8,
          padding: '8px 16px', fontFamily: 'ui-monospace, Menlo, monospace',
          fontSize: 11, letterSpacing: '0.1em', cursor: 'pointer'
        }}>← TILLBAKA</button>
      </div>

      {/* Huvud-innehåll */}
      <div style={{ flex: 1, display: 'flex', gap: 0, padding: '16px 20px', overflow: 'hidden', minHeight: 0 }}>

        {/* Vänster: Lag 1 val */}
        <div style={{ width: 200, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 6, overflow: 'auto', paddingRight: 12 }}>
          <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9, letterSpacing: '0.2em', color: team0.accent, marginBottom: 4, fontWeight: 700 }}>
            ◀ DITT LAG
          </div>
          {TEAM_ROSTER.map((t, i) => (
            <TeamCard key={t.slug} team={t} selected={team0idx === i} onSelect={() => setTeam0idx(i)} side={0} />
          ))}
        </div>

        {/* Mitten: Lagpaneler + VS */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 12, minWidth: 0, paddingLeft: 8 }}>

          {/* Lagpaneler */}
          <div style={{ flex: 1, display: 'flex', gap: 16, minHeight: 0, overflow: 'hidden' }}>
            <TeamPanel team={team0} roster={roster0} side={0} label="DITT LAG" />
            <VSBadge team0={team0} team1={team1} />
            <TeamPanel team={team1} roster={roster1} side={1} label="MOTSTÅNDARE" />
          </div>

          {/* AI-val + start-knapp */}
          <div style={{
            flexShrink: 0, background: 'rgba(0,0,0,0.4)',
            border: '1px solid rgba(255,255,255,0.1)',
            borderRadius: 10, padding: '14px 18px',
            display: 'flex', alignItems: 'center', gap: 16
          }}>
            {/* AI-val lag 1 */}
            <div style={{ flex: 1 }}>
              <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9, letterSpacing: '0.15em', color: 'rgba(255,255,255,0.4)', marginBottom: 6 }}>
                AI-STYRKA DITT LAG
              </div>
              <AISelect opponents={opponents} groupedOpponents={groupedOpponents} value={oppIdx0} onChange={setOppIdx0} accent={team0.accent} />
            </div>
            {/* Start-knapp */}
            <button
              onClick={() => onStart({ team0, team1, oppIdx0, oppIdx1, roster0, roster1 })}
              style={{
                flexShrink: 0,
                background: `linear-gradient(135deg, ${team0.accent}, ${team1.accent})`,
                color: '#000',
                border: 'none',
                borderRadius: 10,
                padding: '14px 32px',
                fontFamily: 'ui-monospace, Menlo, monospace',
                fontWeight: 900, fontSize: 14, letterSpacing: '0.15em',
                cursor: 'pointer',
                boxShadow: `0 0 30px ${team0.accent}55, 0 0 30px ${team1.accent}55`,
                transition: 'transform 0.1s, box-shadow 0.1s'
              }}
              onMouseEnter={e => { e.currentTarget.style.transform = 'scale(1.05)'; }}
              onMouseLeave={e => { e.currentTarget.style.transform = 'scale(1)'; }}
            >
              ▶ SPARKA AV
            </button>
            {/* AI-val lag 2 */}
            <div style={{ flex: 1 }}>
              <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9, letterSpacing: '0.15em', color: 'rgba(255,255,255,0.4)', marginBottom: 6, textAlign: 'right' }}>
                AI-STYRKA MOTSTÅNDARE
              </div>
              <AISelect opponents={opponents} groupedOpponents={groupedOpponents} value={oppIdx1} onChange={setOppIdx1} accent={team1.accent} />
            </div>
          </div>
        </div>

        {/* Höger: Lag 2 val */}
        <div style={{ width: 200, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 6, overflow: 'auto', paddingLeft: 12 }}>
          <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9, letterSpacing: '0.2em', color: team1.accent, marginBottom: 4, fontWeight: 700, textAlign: 'right' }}>
            MOTSTÅNDARE ▶
          </div>
          {TEAM_ROSTER.map((t, i) => (
            <TeamCard key={t.slug} team={t} selected={team1idx === i} onSelect={() => setTeam1idx(i)} side={1} />
          ))}
        </div>
      </div>
    </div>
  );
}

// ── Enkel AI-selector ─────────────────────────────────────────────────────────
function AISelect({ opponents, groupedOpponents, value, onChange, accent }) {
  if (!opponents.length) return (
    <div style={{ color: 'rgba(255,255,255,0.3)', fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 11 }}>
      Laddar…
    </div>
  );
  return (
    <select
      value={value}
      onChange={e => onChange(Number(e.target.value))}
      style={{
        width: '100%', background: 'rgba(255,255,255,0.07)',
        color: '#f3f4f6', border: `1px solid ${accent}55`,
        borderRadius: 6, padding: '8px 10px',
        fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 11,
        cursor: 'pointer', outline: 'none'
      }}
    >
      {groupedOpponents.map(g => (
        <optgroup key={g.version} label={g.version.toUpperCase()}>
          {g.champion && (
            <option value={g.champion.idx}>{g.champion.label || g.champion.name} ★</option>
          )}
          {g.others.map(o => (
            <option key={o.idx} value={o.idx}>{o.label || o.name}</option>
          ))}
        </optgroup>
      ))}
    </select>
  );
}

// ── Förbättrad LineupScreen (används för quiz-matcher) ────────────────────────
function LineupScreen({ matchData, onKickoff }) {
  const [roster, setRoster] = useStateTS(null);
  const [tab, setTab] = useStateTS('roster'); // roster | desc

  useEffectTS(() => {
    if (!matchData.team) { onKickoff(); return; }
    fetchRoster(matchData.team).then(r => setRoster(r) || onKickoff());
  }, []);

  const teamInfo = TEAM_ROSTER.find(t => t.slug === matchData.team) || {
    primary: matchData.color || '#2d8a4e', secondary: '#1a1a1a',
    accent: '#4ecdc4', name: matchData.team
  };

  return (
    <div style={{
      position: 'absolute', inset: 0,
      background: `linear-gradient(160deg, ${teamInfo.primary} 0%, #0a0a0f 55%)`,
      display: 'flex', flexDirection: 'column', overflow: 'hidden'
    }}>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.06, pointerEvents: 'none' }}>
        <line x1="50%" y1="0" x2="50%" y2="100%" stroke="#fff" strokeWidth="2" />
        <circle cx="50%" cy="50%" r="80" fill="none" stroke="#fff" strokeWidth="2" />
      </svg>

      <div style={{ position: 'relative', maxWidth: 600, width: '100%', margin: '0 auto', padding: '28px 24px', display: 'flex', flexDirection: 'column', height: '100%' }}>

        {/* Laglogga + namn */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 18, marginBottom: 20, flexShrink: 0 }}>
          <div style={{ position: 'relative' }}>
            <div style={{ position: 'absolute', inset: -10, background: `radial-gradient(${teamInfo.accent}44, transparent 70%)` }} />
            <img src={`data/teams/${matchData.team}/logo.svg`}
              style={{ width: 80, height: 80, objectFit: 'contain', position: 'relative',
                filter: `drop-shadow(0 0 16px ${teamInfo.accent}88)` }}
              alt="" />
          </div>
          <div>
            <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10, letterSpacing: '0.2em', color: teamInfo.accent }}>
              {matchData.level?.toUpperCase()} · MOTSTÅNDARE
            </div>
            <div style={{ fontFamily: 'Georgia, serif', fontSize: 30, fontWeight: 700, color: '#fff', lineHeight: 1.1, marginTop: 4 }}>
              {teamInfo.name}
            </div>
          </div>
        </div>

        {/* Tabs */}
        {roster && (
          <div style={{ display: 'flex', gap: 0, marginBottom: 16, flexShrink: 0, borderBottom: '1px solid rgba(255,255,255,0.1)' }}>
            {[['roster', 'TRUPP'], ['desc', 'OM LAGET']].map(([key, label]) => (
              <button key={key} onClick={() => setTab(key)} style={{
                background: 'transparent', border: 'none', cursor: 'pointer',
                padding: '8px 20px', fontFamily: 'ui-monospace, Menlo, monospace',
                fontSize: 11, fontWeight: 700, letterSpacing: '0.12em',
                color: tab === key ? teamInfo.accent : 'rgba(255,255,255,0.4)',
                borderBottom: `2px solid ${tab === key ? teamInfo.accent : 'transparent'}`,
                marginBottom: -1, transition: 'color 0.15s'
              }}>{label}</button>
            ))}
          </div>
        )}

        {/* Innehåll */}
        <div style={{ flex: 1, overflow: 'auto', minHeight: 0 }}>
          {!roster && (
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: 120,
              color: 'rgba(255,255,255,0.3)', fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 11, letterSpacing: '0.15em' }}>
              LADDAR LAG…
            </div>
          )}

          {roster && tab === 'roster' && roster.players.map(p => (
            <PlayerRow key={p.id} player={p} team={teamInfo} compact={false} />
          ))}

          {roster && tab === 'desc' && roster.description && (
            <div style={{
              fontFamily: 'Georgia, serif', fontSize: 16, lineHeight: 1.7,
              color: 'rgba(255,255,255,0.85)', fontStyle: 'italic',
              padding: '12px 0'
            }}>
              "{roster.description}"
            </div>
          )}
        </div>

        {/* Kickoff */}
        <button onClick={onKickoff} style={{
          flexShrink: 0, marginTop: 20, width: '100%',
          background: `linear-gradient(90deg, ${teamInfo.primary}, ${teamInfo.accent}99)`,
          color: '#fff', border: `2px solid ${teamInfo.accent}`,
          borderRadius: 8, padding: '16px 24px',
          fontFamily: 'ui-monospace, Menlo, monospace', fontWeight: 700,
          fontSize: 13, letterSpacing: '0.2em', cursor: 'pointer',
          boxShadow: `0 0 24px ${teamInfo.accent}44`
        }}>
          ▶ SPARKA AV
        </button>
      </div>
    </div>
  );
}

window.TeamSelectScreen = TeamSelectScreen;
window.LineupScreen = LineupScreen;
window.TEAM_ROSTER = TEAM_ROSTER;
