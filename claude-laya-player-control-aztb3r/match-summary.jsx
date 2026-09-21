const { useState: useStateMS, useMemo: useMemoMS } = React;

const MS_FONT = "'ui-monospace','Menlo','Courier New',monospace";
const MS_BG   = '#0d0d0d';
const MS_BORDER = '#1a3a1a';
const MS_DIM    = 'rgba(255,255,255,0.35)';

function pct(v) { return Math.round(v || 0) + '%'; }
function px(v)  { return Math.round(v || 0); }

// ── StatBar ────────────────────────────────────────────────────────────────────
function StatBar({ label, v0, v1, color0, color1, unit, sub0, sub1 }) {
  const total = (v0 || 0) + (v1 || 0) || 1;
  const w0 = (v0 || 0) / total * 100;
  const w1 = (v1 || 0) / total * 100;
  const fmt = unit === '%' ? pct : px;
  return (
    <div style={{ display:'flex', alignItems:'center', gap:8, padding:'5px 14px',
      borderBottom:`1px solid rgba(255,255,255,0.04)` }}>
      {/* left value */}
      <div style={{ width:52, textAlign:'right', fontFamily:MS_FONT, fontSize:15,
        fontWeight:'bold', color:color0 }}>
        {fmt(v0)}
        {sub0 != null && (
          <span style={{ display:'block', fontSize:8, color:MS_DIM, fontWeight:'normal' }}>
            {sub0} mål
          </span>
        )}
      </div>
      {/* label + bar */}
      <div style={{ flex:1 }}>
        <div style={{ textAlign:'center', fontFamily:MS_FONT, fontSize:8,
          letterSpacing:'0.12em', color:MS_DIM, marginBottom:3 }}>
          {label}
        </div>
        <div style={{ display:'flex', height:10, borderRadius:3, overflow:'hidden',
          background:'#111' }}>
          <div style={{ width:`${w0}%`, background:color0, transition:'width 0.4s', borderRadius:'3px 0 0 3px' }} />
          <div style={{ width:`${w1}%`, background:color1, transition:'width 0.4s', borderRadius:'0 3px 3px 0' }} />
        </div>
      </div>
      {/* right value */}
      <div style={{ width:52, textAlign:'left', fontFamily:MS_FONT, fontSize:15,
        fontWeight:'bold', color:color1 }}>
        {fmt(v1)}
        {sub1 != null && (
          <span style={{ display:'block', fontSize:8, color:MS_DIM, fontWeight:'normal' }}>
            {sub1} mål
          </span>
        )}
      </div>
    </div>
  );
}

// ── GoalList ───────────────────────────────────────────────────────────────────
function GoalList({ team, color, events, playerGoals, playerAssists, roster }) {
  const players = roster?.players || [];
  const name = id => players.find(p => p.id === id)?.name || `#${id}`;

  const scorers = Object.entries(playerGoals || {})
    .filter(([id]) => {
      const pl = players.find(p => p.id === Number(id));
      return pl != null;
    })
    .map(([id, goals]) => ({ id: Number(id), goals, assists: playerAssists?.[id] || 0 }))
    .filter(e => e.goals > 0);

  const assists = Object.entries(playerAssists || {})
    .filter(([id]) => {
      const pl = players.find(p => p.id === Number(id));
      return pl != null;
    })
    .map(([id, assists]) => ({ id: Number(id), assists }))
    .filter(e => e.assists > 0 && !scorers.find(s => s.id === e.id));

  const goalEvents = events
    .filter(e => e.type === 'goal')
    .map(e => ({ minute: e.minute, scorer: e.scorer, assister: e.assister, kind: e.kind }));

  return (
    <div style={{ flex:1, padding:'8px 12px' }}>
      <div style={{ fontFamily:MS_FONT, fontSize:9, letterSpacing:'0.18em',
        color, marginBottom:6, opacity:0.8 }}>
        {team?.name?.toUpperCase() || 'LAG'}
      </div>
      {goalEvents.map((ev, i) => (
        <div key={i} style={{ fontFamily:MS_FONT, fontSize:11, color:'#e0e0e0',
          marginBottom:3, display:'flex', alignItems:'baseline', gap:6 }}>
          <span style={{ color, minWidth:30, textAlign:'right', fontSize:10 }}>{ev.minute}'</span>
          <span>⚽</span>
          <span>{ev.scorer != null ? name(ev.scorer) : '?'}</span>
          {ev.assister != null && (
            <span style={{ color:MS_DIM, fontSize:9 }}>▸ {name(ev.assister)}</span>
          )}
          {ev.kind === 'freekick' && (
            <span style={{ fontSize:8, color:'#ffd43b', marginLeft:2 }}>FK</span>
          )}
          {ev.kind === 'corner' && (
            <span style={{ fontSize:8, color:'#74c0fc', marginLeft:2 }}>HRN</span>
          )}
        </div>
      ))}
      {assists.length > 0 && (
        <div style={{ marginTop:4, color:MS_DIM, fontFamily:MS_FONT, fontSize:9 }}>
          {assists.map(a => `${name(a.id)} (${a.assists}A)`).join(' · ')}
        </div>
      )}
      {goalEvents.length === 0 && (
        <div style={{ color:'rgba(255,255,255,0.18)', fontFamily:MS_FONT, fontSize:10 }}>—</div>
      )}
    </div>
  );
}

// ── Timeline ───────────────────────────────────────────────────────────────────
function Timeline({ log, color0, color1, roster0, roster1 }) {
  const goals = (log || []).filter(e => e.type === 'goal');
  const players0 = roster0?.players || [];
  const players1 = roster1?.players || [];
  const name = (team, id) => {
    const roster = team === 0 ? players0 : players1;
    const p = roster.find(p => p.id === id);
    if (!p) return `#${id}`;
    const parts = (p.name || '').split(' ');
    return parts[parts.length - 1].slice(0, 4).toUpperCase();
  };

  return (
    <div style={{ padding:'0 14px 10px' }}>
      <div style={{ fontFamily:MS_FONT, fontSize:9, letterSpacing:'0.18em',
        color:'#2a5a2a', marginBottom:10, textAlign:'center' }}>
        M Å L T I D L I N J E
      </div>
      {/* axis */}
      <div style={{ position:'relative', height:52 }}>
        {/* bar */}
        <div style={{ position:'absolute', top:22, left:0, right:0, height:6,
          background:'#161f16', borderRadius:3 }} />
        {/* HT marker */}
        <div style={{ position:'absolute', top:13, left:'50%', transform:'translateX(-50%)',
          width:1, height:20, background:'#1e3a1e' }} />
        <div style={{ position:'absolute', top:34, left:'50%', transform:'translateX(-50%)',
          fontFamily:MS_FONT, fontSize:7, color:'#2a4a2a' }}>HT</div>
        {/* 0' and 90' */}
        <div style={{ position:'absolute', top:35, left:0,
          fontFamily:MS_FONT, fontSize:8, color:'#2a4a2a' }}>0'</div>
        <div style={{ position:'absolute', top:35, right:0,
          fontFamily:MS_FONT, fontSize:8, color:'#2a4a2a' }}>90'</div>

        {goals.map((ev, i) => {
          const xPct = Math.min(98, Math.max(2, (ev.minute / 90) * 100));
          const color = ev.team === 0 ? color0 : color1;
          const bg    = ev.team === 0 ? color0 + '55' : color1 + '55';
          return (
            <div key={i} style={{ position:'absolute', left:`${xPct}%`,
              transform:'translateX(-50%)', top:0, display:'flex',
              flexDirection:'column', alignItems:'center', gap:1 }}>
              <div style={{ fontFamily:MS_FONT, fontSize:8, color, whiteSpace:'nowrap' }}>
                {ev.minute}'
              </div>
              <div style={{ width:12, height:12, borderRadius:'50%',
                background:bg, border:`2px solid ${color}`,
                display:'flex', alignItems:'center', justifyContent:'center',
                fontSize:7 }}>⚽</div>
              <div style={{ fontFamily:MS_FONT, fontSize:7, color,
                whiteSpace:'nowrap', marginTop:2 }}>
                {ev.scorer != null ? name(ev.team, ev.scorer) : '?'}
              </div>
            </div>
          );
        })}
      </div>
      {goals.length === 0 && (
        <div style={{ textAlign:'center', color:'rgba(255,255,255,0.18)',
          fontFamily:MS_FONT, fontSize:10, marginTop:4 }}>
          Inga mål
        </div>
      )}
    </div>
  );
}

// ── MatchSummary ───────────────────────────────────────────────────────────────
function MatchSummary({ score, stats, log, playerGoals, playerAssists,
  team0, team1, roster0, roster1, onContinue, onExit }) {

  const color0 = team0?.primary || '#3464a8';
  const color1 = team1?.primary || '#c82828';
  const s = stats || {};

  const goalEvents0 = (log || []).filter(e => e.type === 'goal' && e.team === 0);
  const goalEvents1 = (log || []).filter(e => e.type === 'goal' && e.team === 1);

  // split playerGoals/assists by team via roster
  const players0 = roster0?.players || [];
  const players1 = roster1?.players || [];
  const ids0 = new Set(players0.map(p => p.id));
  const ids1 = new Set(players1.map(p => p.id));

  const pGoals0 = Object.fromEntries(
    Object.entries(playerGoals || {}).filter(([id]) => ids0.has(Number(id)))
  );
  const pGoals1 = Object.fromEntries(
    Object.entries(playerGoals || {}).filter(([id]) => ids1.has(Number(id)))
  );
  const pAssists0 = Object.fromEntries(
    Object.entries(playerAssists || {}).filter(([id]) => ids0.has(Number(id)))
  );
  const pAssists1 = Object.fromEntries(
    Object.entries(playerAssists || {}).filter(([id]) => ids1.has(Number(id)))
  );

  return (
    <div style={{
      position:'absolute', inset:0,
      background: MS_BG,
      display:'flex', flexDirection:'column',
      fontFamily: MS_FONT,
      overflowY: 'auto',
      zIndex: 100,
    }}>
      {/* scanline overlay */}
      <div style={{
        position:'fixed', inset:0, pointerEvents:'none', zIndex:200,
        backgroundImage:'repeating-linear-gradient(0deg,rgba(0,0,0,0.12) 0px,rgba(0,0,0,0.12) 1px,transparent 1px,transparent 4px)',
      }} />

      <div style={{ maxWidth:660, margin:'0 auto', width:'100%', padding:'0 4px 20px' }}>

        {/* ── Header ── */}
        <div style={{
          background:'#0f1f0f',
          borderBottom:`2px solid ${MS_BORDER}`,
          padding:'14px 16px 10px',
          textAlign:'center',
        }}>
          <div style={{ fontSize:8, letterSpacing:'0.5em', color:'#3a6a3a', marginBottom:6 }}>
            F U L L T I D
          </div>
          <div style={{ display:'flex', alignItems:'center', justifyContent:'center', gap:16 }}>
            <div style={{ flex:1, textAlign:'right', fontSize:16, fontWeight:'bold',
              color:color0, letterSpacing:'0.05em' }}>
              {team0?.name || 'BLÅLAG'}
            </div>
            <div style={{ fontSize:32, fontWeight:'bold', color:'#f0f0f0',
              letterSpacing:'0.12em', minWidth:100, textAlign:'center' }}>
              {score?.[0] ?? 0} – {score?.[1] ?? 0}
            </div>
            <div style={{ flex:1, textAlign:'left', fontSize:16, fontWeight:'bold',
              color:color1, letterSpacing:'0.05em' }}>
              {team1?.name || 'RÖDLAG'}
            </div>
          </div>
        </div>

        {/* ── Stats ── */}
        <div style={{ background:'#0c140c', borderBottom:`1px solid ${MS_BORDER}` }}>
          <div style={{ textAlign:'center', padding:'6px 0 2px',
            fontSize:8, letterSpacing:'0.4em', color:'#2a5a2a' }}>
            S T A T I S T I K
          </div>
          <StatBar label="SKOTT PÅ MÅL"  v0={s.shots?.[0]}        v1={s.shots?.[1]}        color0={color0} color1={color1} />
          <StatBar label="MÅL"           v0={score?.[0]}          v1={score?.[1]}          color0={color0} color1={color1} />
          <StatBar label="FRISPARKAR"    v0={s.freeKicks?.[0]}    v1={s.freeKicks?.[1]}    color0={color0} color1={color1}
            sub0={s.freeKickGoals?.[0]} sub1={s.freeKickGoals?.[1]} />
          <StatBar label="HÖRNOR"        v0={s.corners?.[0]}      v1={s.corners?.[1]}      color0={color0} color1={color1}
            sub0={s.cornerGoals?.[0]} sub1={s.cornerGoals?.[1]} />
          <StatBar label="BOLLINNEHAV"   v0={s.possession?.[0]}   v1={s.possession?.[1]}   color0={color0} color1={color1} unit="%" />
          <StatBar label="INNEHAV EGET HALF"   v0={s.possOwnHalf?.[0]}  v1={s.possOwnHalf?.[1]}  color0={color0 + 'aa'} color1={color1 + 'aa'} unit="%" />
          <StatBar label="INNEHAV MOTST. HALF" v0={s.possOppHalf?.[0]}  v1={s.possOppHalf?.[1]}  color0={color0} color1={color1} unit="%" />
        </div>

        {/* ── Scorers ── */}
        <div style={{ background:'#0c120c', borderBottom:`1px solid ${MS_BORDER}` }}>
          <div style={{ textAlign:'center', padding:'6px 0 2px',
            fontSize:8, letterSpacing:'0.4em', color:'#2a5a2a' }}>
            M Å L G Ö R A R E
          </div>
          <div style={{ display:'flex' }}>
            <div style={{ flex:1, borderRight:`1px solid ${MS_BORDER}` }}>
              <GoalList team={team0} color={color0} events={goalEvents0}
                playerGoals={pGoals0} playerAssists={pAssists0} roster={roster0} />
            </div>
            <div style={{ flex:1 }}>
              <GoalList team={team1} color={color1} events={goalEvents1}
                playerGoals={pGoals1} playerAssists={pAssists1} roster={roster1} />
            </div>
          </div>
        </div>

        {/* ── Timeline ── */}
        <div style={{ background:'#0c120c', borderBottom:`1px solid ${MS_BORDER}` }}>
          <Timeline log={log} color0={color0} color1={color1}
            roster0={roster0} roster1={roster1} />
        </div>

        {/* ── Buttons ── */}
        <div style={{ display:'flex', gap:10, padding:'14px 14px 4px',
          justifyContent:'center' }}>
          <button onClick={onContinue}
            style={{
              padding:'12px 40px', fontFamily:MS_FONT, fontSize:13,
              letterSpacing:'0.15em', background:'#0f200f',
              color:'#4aaa4a', border:`1px solid #2a6a2a`,
              borderRadius:4, cursor:'pointer',
            }}>
            [ FORTSÄTT ]
          </button>
          <button onClick={onExit}
            style={{
              padding:'12px 24px', fontFamily:MS_FONT, fontSize:11,
              letterSpacing:'0.1em', background:'transparent',
              color:MS_DIM, border:`1px solid rgba(255,255,255,0.1)`,
              borderRadius:4, cursor:'pointer',
            }}>
            AVSLUTA
          </button>
        </div>
      </div>
    </div>
  );
}

window.MatchSummary = MatchSummary;
