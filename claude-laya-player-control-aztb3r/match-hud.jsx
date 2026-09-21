// ── Förbättrad Match-HUD overlay ─────────────────────────────────────────────
const { useState: useStateHUD, useEffect: useEffectHUD, useRef: useRefHUD } = React;

// ── Live stats-bar (skott + bollinnehav) ──────────────────────────────────────
function LiveStatsBar({ stats, t0, t1 }) {
  if (!stats) return null;
  const totalShots = (stats.shots[0] || 0) + (stats.shots[1] || 0);
  const poss0 = stats.possession[0] || 50;

  return (
    <div style={{
      position: 'absolute', bottom: 40, left: '50%', transform: 'translateX(-50%)',
      zIndex: 30, display: 'flex', flexDirection: 'column', gap: 5,
      background: 'rgba(0,0,0,0.72)', border: '1px solid rgba(255,255,255,0.1)',
      borderRadius: 8, padding: '8px 16px', minWidth: 280,
      backdropFilter: 'blur(10px)'
    }}>
      {/* Bollinnehav */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
          color: t0?.accent || '#6ea8fe', fontWeight: 700, width: 28, textAlign: 'right' }}>
          {Math.round(poss0)}%
        </div>
        <div style={{ flex: 1, height: 4, background: 'rgba(255,255,255,0.1)', borderRadius: 2, overflow: 'hidden', position: 'relative' }}>
          <div style={{ position: 'absolute', left: 0, top: 0, height: '100%',
            width: `${poss0}%`, background: t0?.accent || '#6ea8fe',
            transition: 'width 0.5s ease', borderRadius: 2 }} />
          <div style={{ position: 'absolute', right: 0, top: 0, height: '100%',
            width: `${100 - poss0}%`, background: t1?.accent || '#f87171',
            transition: 'width 0.5s ease', borderRadius: 2 }} />
        </div>
        <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
          color: t1?.accent || '#f87171', fontWeight: 700, width: 28 }}>
          {Math.round(100 - poss0)}%
        </div>
        <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 8,
          color: 'rgba(255,255,255,0.3)', letterSpacing: '0.1em', marginLeft: 2 }}>BOLL</div>
      </div>
      {/* Skott */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
          color: t0?.accent || '#6ea8fe', fontWeight: 700, width: 28, textAlign: 'right' }}>
          {stats.shots[0] || 0}
        </div>
        <div style={{ flex: 1, height: 4, background: 'rgba(255,255,255,0.1)', borderRadius: 2, overflow: 'hidden' }}>
          {totalShots > 0 && (
            <div style={{ height: '100%',
              width: `${((stats.shots[0]||0) / totalShots) * 100}%`,
              background: t0?.accent || '#6ea8fe', transition: 'width 0.4s ease', borderRadius: 2 }} />
          )}
        </div>
        <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
          color: t1?.accent || '#f87171', fontWeight: 700, width: 28 }}>
          {stats.shots[1] || 0}
        </div>
        <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 8,
          color: 'rgba(255,255,255,0.3)', letterSpacing: '0.1em', marginLeft: 2 }}>SKOTT</div>
      </div>
    </div>
  );
}

// ── Fulltime matchstatistik ───────────────────────────────────────────────────
function FulltimeStats({ stats, score, t0, t1, onClose }) {
  const StatRow = ({ label, v0, v1 }) => {
    const total = (v0 || 0) + (v1 || 0);
    const pct0 = total > 0 ? ((v0||0)/total)*100 : 50;
    return (
      <div style={{ marginBottom: 10 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 3 }}>
          <span style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 11,
            fontWeight: 700, color: t0?.accent || '#6ea8fe' }}>{v0 || 0}</span>
          <span style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
            color: 'rgba(255,255,255,0.35)', letterSpacing: '0.12em' }}>{label}</span>
          <span style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 11,
            fontWeight: 700, color: t1?.accent || '#f87171' }}>{v1 || 0}</span>
        </div>
        <div style={{ height: 5, background: 'rgba(255,255,255,0.08)', borderRadius: 3, overflow: 'hidden', display: 'flex' }}>
          <div style={{ width: `${pct0}%`, background: t0?.accent || '#6ea8fe',
            transition: 'width 0.8s cubic-bezier(0.34,1.2,0.64,1)', borderRadius: '3px 0 0 3px' }} />
          <div style={{ flex: 1, background: t1?.accent || '#f87171',
            borderRadius: '0 3px 3px 0' }} />
        </div>
      </div>
    );
  };

  const won = score[0] > score[1];
  const draw = score[0] === score[1];

  return (
    <div style={{
      position: 'absolute', inset: 0, zIndex: 50,
      background: 'rgba(0,0,0,0.88)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      backdropFilter: 'blur(8px)'
    }}>
      <div style={{
        width: 'min(520px, 92%)', background: '#0e0e14',
        border: '1px solid rgba(255,255,255,0.12)',
        borderRadius: 14, overflow: 'hidden',
        boxShadow: '0 24px 80px rgba(0,0,0,0.7)'
      }}>
        {/* Header med slutresultat */}
        <div style={{
          background: won
            ? `linear-gradient(135deg, ${t0?.primary||'#1a3a6e'}, ${t0?.secondary||'#0d2040'})`
            : draw
              ? 'linear-gradient(135deg, #1a1a2e, #0e0e18)'
              : `linear-gradient(135deg, ${t1?.primary||'#5a0e0e'}, #0e0e18)`,
          padding: '22px 24px', textAlign: 'center'
        }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 20, marginBottom: 8 }}>
            {t0?.slug && <img src={`data/teams/${t0.slug}/logo.svg`}
              style={{ width: 44, height: 44, objectFit: 'contain', filter: `drop-shadow(0 0 8px ${t0.accent}88)` }} alt="" />}
            <div>
              <div style={{ fontFamily: 'Georgia, serif', fontSize: 44, fontWeight: 900,
                color: '#fff', lineHeight: 1, letterSpacing: '-0.02em' }}>
                {score[0]} – {score[1]}
              </div>
              <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 12,
                color: won ? '#86efac' : draw ? '#e2e8f0' : '#fca5a5',
                letterSpacing: '0.2em', marginTop: 4, fontWeight: 700 }}>
                {won ? 'VINST' : draw ? 'OAVGJORT' : 'FÖRLUST'}
              </div>
            </div>
            {t1?.slug && <img src={`data/teams/${t1.slug}/logo.svg`}
              style={{ width: 44, height: 44, objectFit: 'contain', filter: `drop-shadow(0 0 8px ${t1.accent}88)` }} alt="" />}
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-around' }}>
            <div style={{ fontFamily: 'Georgia, serif', fontSize: 13, color: t0?.accent || '#6ea8fe' }}>{t0?.name || 'LAG 1'}</div>
            <div style={{ fontFamily: 'Georgia, serif', fontSize: 13, color: t1?.accent || '#f87171' }}>{t1?.name || 'LAG 2'}</div>
          </div>
        </div>

        {/* Statistik */}
        <div style={{ padding: '20px 24px' }}>
          <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
            letterSpacing: '0.2em', color: 'rgba(255,255,255,0.3)', marginBottom: 14 }}>
            MATCHSTATISTIK
          </div>
          <StatRow label="SKOTT"       v0={stats?.shots?.[0]}      v1={stats?.shots?.[1]} />
          <StatRow label="PASSNINGAR"  v0={stats?.passes?.[0]}     v1={stats?.passes?.[1]} />
          <StatRow label="TACKLINGAR"  v0={stats?.tackles?.[0]}    v1={stats?.tackles?.[1]} />
          <StatRow label="HÖRNSPARKAR" v0={stats?.corners?.[0]}    v1={stats?.corners?.[1]} />
          <StatRow label="BOLLINNEHAV" v0={Math.round(stats?.possession?.[0]||50)} v1={Math.round(stats?.possession?.[1]||50)} />
        </div>

        <div style={{ padding: '0 24px 20px', display: 'flex', justifyContent: 'center' }}>
          <button onClick={onClose} style={{
            background: 'rgba(255,255,255,0.08)', color: '#fff',
            border: '1px solid rgba(255,255,255,0.15)', borderRadius: 8,
            padding: '10px 28px', fontFamily: 'ui-monospace, Menlo, monospace',
            fontSize: 11, letterSpacing: '0.15em', cursor: 'pointer',
            transition: 'background 0.15s'
          }}
          onMouseEnter={e => e.currentTarget.style.background = 'rgba(255,255,255,0.14)'}
          onMouseLeave={e => e.currentTarget.style.background = 'rgba(255,255,255,0.08)'}
          >
            STÄNG
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Huvud MatchHUD ────────────────────────────────────────────────────────────
function MatchHUD({ game, team0info, team1info, onExit, matchStats }) {
  const [goalEvent, setGoalEvent] = useStateHUD(null);
  const prevScoreRef = useRefHUD([0, 0]);
  const [showFulltimeStats, setShowFulltimeStats] = useStateHUD(false);

  // Visa fulltime-stats automatiskt
  useEffectHUD(() => {
    if (game?.phase === 'fulltime') {
      const t = setTimeout(() => setShowFulltimeStats(true), 3600);
      return () => clearTimeout(t);
    }
  }, [game?.phase]);

  // Detektera mål
  useEffectHUD(() => {
    if (!game) return;
    const s = game.score;
    const prev = prevScoreRef.current;
    if (s[0] !== prev[0]) {
      setGoalEvent({ text: 'MÅÅÅL!', sub: team0info?.name || 'LAG 1', color: team0info?.accent || '#4ecdc4' });
      setTimeout(() => setGoalEvent(null), 2800);
    } else if (s[1] !== prev[1]) {
      setGoalEvent({ text: 'MÅÅÅL!', sub: team1info?.name || 'LAG 2', color: team1info?.accent || '#f472b6' });
      setTimeout(() => setGoalEvent(null), 2800);
    }
    prevScoreRef.current = [...s];
  }, [game?.score?.[0], game?.score?.[1]]);

  if (!game) return null;

  const totalSecs = game.timer > 0 ? Math.ceil(game.timer / 60) : 0;
  const elapsed = Math.max(0, 150 - totalSecs);
  const mins = Math.floor(elapsed / 60);
  const secs = elapsed % 60;
  const timeStr = `${String(mins).padStart(2,'0')}:${String(secs).padStart(2,'0')}`;
  const progress = elapsed / 150;

  const t0 = team0info || { name: 'LAG 1', accent: '#6ea8fe', primary: '#1e3a5f', secondary: '#0d2040', slug: null };
  const t1 = team1info || { name: 'LAG 2', accent: '#f87171', primary: '#5f1e1e', secondary: '#3f0e0e', slug: null };

  return (
    <>
      {/* ── Scoreboard ── */}
      <div style={{
        position: 'absolute', top: 12, left: '50%', transform: 'translateX(-50%)',
        zIndex: 30, display: 'flex', alignItems: 'stretch',
        background: 'rgba(8,8,14,0.94)',
        border: '1px solid rgba(255,255,255,0.1)',
        borderRadius: 10, backdropFilter: 'blur(14px)',
        boxShadow: '0 4px 28px rgba(0,0,0,0.65)', overflow: 'hidden',
        minWidth: 360
      }}>
        {/* Lag 0 */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '7px 12px',
          background: `linear-gradient(90deg, ${t0.primary}aa, transparent)`,
          flex: 1, justifyContent: 'flex-end' }}>
          <div style={{ textAlign: 'right' }}>
            <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
              letterSpacing: '0.1em', color: t0.accent, fontWeight: 700 }}>DU</div>
            <div style={{ fontFamily: 'Georgia, serif', fontSize: 12, color: '#f3f4f6',
              fontWeight: 700, whiteSpace: 'nowrap' }}>{t0.name}</div>
          </div>
          {t0.slug && (
            <img src={`data/teams/${t0.slug}/logo.svg`}
              style={{ width: 28, height: 28, objectFit: 'contain',
                filter: `drop-shadow(0 0 5px ${t0.accent}88)` }} alt="" />
          )}
        </div>

        {/* Score + timer */}
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center',
          justifyContent: 'center', padding: '6px 14px',
          background: 'rgba(0,0,0,0.55)', flexShrink: 0, minWidth: 100 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8,
            fontFamily: 'Georgia, serif', fontWeight: 900, fontSize: 26, color: '#fff', lineHeight: 1 }}>
            <span style={{ color: game.score[0] > game.score[1] ? t0.accent : '#fff' }}>{game.score[0]}</span>
            <span style={{ color: 'rgba(255,255,255,0.25)', fontSize: 18 }}>–</span>
            <span style={{ color: game.score[1] > game.score[0] ? t1.accent : '#fff' }}>{game.score[1]}</span>
          </div>
          <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 10,
            color: game.timer < 1200 ? '#ef4444' : 'rgba(255,255,255,0.55)',
            letterSpacing: '0.1em', marginTop: 1,
            fontWeight: game.timer < 1200 ? 700 : 400 }}>
            {timeStr}
          </div>
          <div style={{ width: 72, height: 2, background: 'rgba(255,255,255,0.1)',
            borderRadius: 2, marginTop: 3 }}>
            <div style={{ height: '100%', width: `${progress * 100}%`, borderRadius: 2,
              background: `linear-gradient(90deg, ${t0.accent}, ${t1.accent})`,
              transition: 'width 0.5s linear' }} />
          </div>
        </div>

        {/* Lag 1 */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '7px 12px',
          background: `linear-gradient(270deg, ${t1.primary}aa, transparent)`,
          flex: 1 }}>
          {t1.slug && (
            <img src={`data/teams/${t1.slug}/logo.svg`}
              style={{ width: 28, height: 28, objectFit: 'contain',
                filter: `drop-shadow(0 0 5px ${t1.accent}88)` }} alt="" />
          )}
          <div>
            <div style={{ fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 9,
              letterSpacing: '0.1em', color: t1.accent, fontWeight: 700 }}>MOTST.</div>
            <div style={{ fontFamily: 'Georgia, serif', fontSize: 12, color: '#f3f4f6',
              fontWeight: 700, whiteSpace: 'nowrap' }}>{t1.name}</div>
          </div>
        </div>
      </div>

      {/* ── Live stats (under canvas HUD-area) ── */}
      {game.phase === 'playing' && matchStats && (
        <LiveStatsBar stats={matchStats} t0={t0} t1={t1} />
      )}

      {/* ── Set-piece text ── */}
      {game.setPieceText && game.setPieceTimer > 0 && (
        <div style={{
          position: 'absolute', top: 68, left: '50%', transform: 'translateX(-50%)',
          zIndex: 31, background: 'rgba(0,0,0,0.8)',
          border: '1px solid rgba(255,215,0,0.25)', borderRadius: 6,
          padding: '5px 16px', fontFamily: 'ui-monospace, Menlo, monospace',
          fontSize: 11, letterSpacing: '0.2em', color: '#fcd34d', fontWeight: 700,
          backdropFilter: 'blur(8px)', whiteSpace: 'nowrap'
        }}>
          {game.setPieceText}
        </div>
      )}

      {/* ── Mål-banner ── */}
      {goalEvent && (
        <div style={{
          position: 'absolute', top: '26%', left: '50%',
          transform: 'translateX(-50%)',
          zIndex: 40, textAlign: 'center',
          animation: 'goalPop 0.35s cubic-bezier(0.34,1.56,0.64,1)'
        }}>
          <div style={{
            fontFamily: 'Georgia, serif', fontSize: 80, fontWeight: 900,
            color: goalEvent.color, lineHeight: 1, letterSpacing: '-0.02em',
            textShadow: `0 0 50px ${goalEvent.color}88, 4px 4px 0 rgba(0,0,0,0.5)`
          }}>{goalEvent.text}</div>
          <div style={{
            fontFamily: 'ui-monospace, Menlo, monospace', fontSize: 15,
            color: '#fff', letterSpacing: '0.2em', marginTop: 4,
            textShadow: '2px 2px 0 rgba(0,0,0,0.8)'
          }}>{goalEvent.sub}</div>
        </div>
      )}

      {/* ── Exit ── */}
      <button onClick={onExit} style={{
        position: 'absolute', bottom: 12, left: 12, zIndex: 30,
        background: 'rgba(0,0,0,0.55)', color: 'rgba(255,255,255,0.45)',
        border: '1px solid rgba(255,255,255,0.1)', borderRadius: 6,
        padding: '5px 10px', fontFamily: 'ui-monospace, Menlo, monospace',
        fontSize: 9, letterSpacing: '0.1em', cursor: 'pointer',
        backdropFilter: 'blur(8px)'
      }}>← ESC</button>

      {/* ── Fulltime stats overlay ── */}
      {showFulltimeStats && (
        <FulltimeStats
          stats={matchStats}
          score={game.score}
          t0={t0} t1={t1}
          onClose={() => setShowFulltimeStats(false)}
        />
      )}

      <style>{`
        @keyframes goalPop {
          from { transform: translateX(-50%) scale(0.3); opacity: 0; }
          to   { transform: translateX(-50%) scale(1);   opacity: 1; }
        }
      `}</style>
    </>
  );
}

window.MatchHUD = MatchHUD;
