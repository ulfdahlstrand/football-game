const { useEffect: useEffFM, useRef: useRefFM, useState: useStateFM, useMemo: useMemoFM } = React;

// ── Konstanter (rendering + initial state; simulering bor i Rust/WASM) ───────
const FW = 880, FH = 520, GH = 130, GD = 26;
const BR = 8;
const GAME_SECS = 150;
const h2 = FH / 2;
const FIELD_LINE = 18;
const GOAL_AREA_W = 54;

const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
const norm  = (dx, dy) => { const m = Math.hypot(dx, dy) || 1; return [dx/m, dy/m]; };

// ── Sprite-ritning (canvas, Pokémon-stil top-down) ───────────────────────────

function drawKnocked(ctx, teamColor, hairColor, knockTimer) {
  const blink = Math.floor(knockTimer / 14) % 2 === 0;
  ctx.save();
  // Skugga
  ctx.fillStyle = 'rgba(0,0,0,0.25)';
  ctx.beginPath(); ctx.ellipse(4, 10, 22, 4, 0, 0, Math.PI*2); ctx.fill();
  // Kropp liggande
  ctx.fillStyle = teamColor;
  ctx.fillRect(-2, -4, 28, 9);
  ctx.fillStyle = 'rgba(255,255,255,0.35)';
  ctx.fillRect(-2, -4, 28, 2);
  ctx.strokeStyle = '#1a1a1a'; ctx.lineWidth = 1.2;
  ctx.strokeRect(-2, -4, 28, 9);
  // Huvud
  ctx.fillStyle = '#f4c896';
  ctx.fillRect(-15, -6, 13, 13);
  ctx.strokeStyle = '#1a1a1a'; ctx.lineWidth = 1.2;
  ctx.strokeRect(-15, -6, 13, 13);
  // Hår
  ctx.fillStyle = hairColor;
  ctx.fillRect(-15, -6, 13, 4);
  // × ögon
  ctx.strokeStyle = '#1a1a1a'; ctx.lineWidth = 1.5;
  [[-13,-1,-10,2],[-10,-1,-13,2],[-7,-1,-4,2],[-4,-1,-7,2]].forEach(([x1,y1,x2,y2]) => {
    ctx.beginPath(); ctx.moveTo(x1,y1); ctx.lineTo(x2,y2); ctx.stroke();
  });
  // Ben
  ctx.fillStyle = '#1a3a7a';
  ctx.fillRect(24, -8, 11, 6);
  ctx.fillRect(24,  3, 11, 6);
  ctx.strokeStyle = '#1a1a1a'; ctx.lineWidth = 1;
  ctx.strokeRect(24, -8, 11, 6); ctx.strokeRect(24, 3, 11, 6);
  // Böjt knä
  ctx.fillStyle = '#1a3a7a';
  ctx.fillRect(32, -12, 6, 16); ctx.strokeRect(32, -12, 6, 16);
  // Arm mot knä
  ctx.fillStyle = '#f4c896';
  ctx.fillRect(6, -12, 5, 9); ctx.strokeRect(6, -12, 5, 9);
  ctx.fillRect(11, -12, 20, 5); ctx.strokeRect(11, -12, 20, 5);
  // Smärtstjärnor
  if (blink) {
    ctx.fillStyle = '#ffd43b'; ctx.font = 'bold 12px serif';
    ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
    ctx.fillText('★', -8, -14);
    ctx.fillStyle = '#ff4757'; ctx.font = '9px sans-serif';
    ctx.fillText('!', 2, -17);
  } else {
    ctx.fillStyle = '#ff6b35'; ctx.font = '10px serif';
    ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
    ctx.fillText('✦', -6, -15);
  }
  ctx.restore();
}

function drawSpriteDown(ctx, teamColor, hairColor, legPhase) {
  const lOff = legPhase ? 3 : -3;
  ctx.fillStyle = '#1a3a7a';
  ctx.fillRect(-5, 10, 4, 7 + lOff); ctx.fillRect(1, 10, 4, 7 - lOff);
  ctx.fillStyle = '#1a1a1a'; ctx.fillRect(-5,17+lOff,4,2); ctx.fillRect(1,17-lOff,4,2);
  ctx.fillStyle = teamColor;
  ctx.fillRect(-8, -3, 16, 13);
  ctx.fillStyle = 'rgba(255,255,255,0.35)'; ctx.fillRect(-8,-3,16,2);
  ctx.fillStyle = '#f4c896';
  ctx.fillRect(-11,-2,3,9); ctx.fillRect(8,-2,3,9);
  ctx.fillRect(-7,-15,14,12);
  ctx.fillStyle = hairColor; ctx.fillRect(-7,-15,14,4);
  ctx.fillStyle = '#1a1a1a';
  ctx.fillRect(-4,-8,2,2); ctx.fillRect(2,-8,2,2);
  ctx.strokeStyle = '#1a1a1a'; ctx.lineWidth = 1;
  ctx.strokeRect(-8,-3,16,13); ctx.strokeRect(-7,-15,14,12);
}

function drawSpriteUp(ctx, teamColor, hairColor, num, legPhase) {
  const lOff = legPhase ? 3 : -3;
  ctx.fillStyle = '#1a3a7a';
  ctx.fillRect(-5,10,4,7+lOff); ctx.fillRect(1,10,4,7-lOff);
  ctx.fillStyle = '#1a1a1a'; ctx.fillRect(-5,17+lOff,4,2); ctx.fillRect(1,17-lOff,4,2);
  ctx.fillStyle = teamColor; ctx.fillRect(-8,-3,16,13);
  ctx.fillStyle = 'rgba(255,255,255,0.35)'; ctx.fillRect(-8,-3,16,2);
  ctx.fillStyle = '#f4c896';
  ctx.fillRect(-11,-2,3,9); ctx.fillRect(8,-2,3,9);
  ctx.fillStyle = hairColor; ctx.fillRect(-7,-15,14,15);
  ctx.fillStyle = '#f4c896'; ctx.fillRect(-7,-1,14,4);
  ctx.fillStyle = 'rgba(255,255,255,0.7)';
  ctx.font = 'bold 7px ui-monospace,monospace'; ctx.textAlign='center'; ctx.textBaseline='middle';
  ctx.fillText(num, 0, 5);
  ctx.strokeStyle = '#1a1a1a'; ctx.lineWidth = 1;
  ctx.strokeRect(-8,-3,16,13); ctx.strokeRect(-7,-15,14,15);
}

function drawSpriteSide(ctx, teamColor, hairColor, facingLeft, legPhase) {
  const flip = facingLeft ? -1 : 1;
  ctx.save(); if (facingLeft) { ctx.scale(-1,1); }
  const lOff = legPhase ? 3 : -3;
  ctx.fillStyle = '#1a3a7a';
  ctx.fillRect(-3,10,6,7+lOff); ctx.fillRect(-3,17+lOff,6,2);
  ctx.fillStyle = teamColor; ctx.fillRect(-7,-3,14,13);
  ctx.fillStyle = 'rgba(255,255,255,0.35)'; ctx.fillRect(-7,-3,14,2);
  ctx.fillStyle = '#f4c896'; ctx.fillRect(5,-2,3,8);
  ctx.fillRect(-6,-15,12,12);
  ctx.fillStyle = hairColor; ctx.fillRect(-6,-15,12,4); ctx.fillRect(-6,-15,3,9);
  ctx.fillStyle = '#1a1a1a'; ctx.fillRect(2,-9,2,2);
  ctx.fillStyle = '#d4a076'; ctx.fillRect(7,-7,1,2);
  ctx.strokeStyle = '#1a1a1a'; ctx.lineWidth = 1;
  ctx.strokeRect(-7,-3,14,13); ctx.strokeRect(-6,-15,12,12);
  ctx.restore();
}

// NES-palett (Stil A — Pixel Retro)
const NES_TEAM0  = '#2040e0';   // blå
const NES_TEAM1  = '#e83030';   // röd
const NES_GK0    = '#f0c030';   // gul GK
const NES_GK1    = '#30c060';   // grön GK

function drawPlayer(ctx, p, game) {
  const hasBall  = game.ball.owner === p.id;
  const baseColors = game.teamColors || { 0: NES_TEAM0, 1: NES_TEAM1 };
  const teamColor = p.role === 'gk'
    ? (p.team === 0 ? NES_GK0 : NES_GK1)
    : baseColors[p.team];
  const hairColor = p.hairColor;
  const celebrateJump = p.celebrateTimer > 0 ? -Math.abs(Math.sin(p.celebrateTimer * 0.18)) * 22 : 0;
  const hopPhase = p.jumpTimer > 0 ? Math.sin((1 - p.jumpTimer/JUMP_DUR) * Math.PI) : 0;
  const jump = celebrateJump - hopPhase * 24;

  ctx.save();
  ctx.translate(p.x, p.y + jump);

  if (p.state === 'knocked') {
    drawKnocked(ctx, teamColor, hairColor, p.knockTimer);
    ctx.restore(); return;
  }

  // GK on ground after missed dive
  if (p.gkDiveTimer < 0) {
    ctx.save();
    ctx.rotate(p.gkDiveDir === 'up' ? -Math.PI/2 : Math.PI/2);
    drawKnocked(ctx, teamColor, hairColor, 1);
    ctx.restore();
    ctx.restore(); return;
  }

  // GK actively diving
  if (p.gkDiveTimer > 0) {
    ctx.save();
    ctx.rotate(p.gkDiveDir === 'up' ? -Math.PI/3 : Math.PI/3);
    ctx.scale(1.2, 0.7);
    drawSpriteSide(ctx, teamColor, hairColor, false, false);
    ctx.restore();
    if (p.human) { ctx.strokeStyle='#ffd43b'; ctx.lineWidth=2.5; ctx.strokeRect(-11,-18,22,38); }
    ctx.restore(); return;
  }

  // Skugga
  ctx.fillStyle = 'rgba(0,0,0,0.2)';
  ctx.beginPath(); ctx.ellipse(1, 15, 10, 3, 0, 0, Math.PI*2); ctx.fill();

  const step = Math.floor(p.stepCounter / 7) % 2 === 0;
  const facing = p.facing || 'down';
  if (facing === 'down')  drawSpriteDown(ctx, teamColor, hairColor, step);
  else if (facing === 'up') drawSpriteUp(ctx, teamColor, hairColor, String(p.id+1), step);
  else drawSpriteSide(ctx, teamColor, hairColor, facing === 'left', step);

  // Gul ring runt spelaren (mänsklig)
  if (p.human) {
    ctx.strokeStyle = '#ffd43b'; ctx.lineWidth = 2.5;
    ctx.strokeRect(-11, -18, 22, 38);
  }
  // Boll-indikator
  if (hasBall) {
    ctx.fillStyle = 'rgba(255,255,255,0.22)';
    ctx.fillRect(-8,-3,16,13);
  }
  // Slow-effekt (gul blink)
  if (p.slowTimer > 0) {
    ctx.fillStyle = 'rgba(255,220,0,0.28)';
    ctx.fillRect(-12,-20,24,42);
  }
  ctx.restore();
}


// ── Match log (skickas till log-server.py om den kör) ────────────────────────
function saveMatchLog(summary) {
  fetch('http://localhost:8766/log', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(summary),
  }).catch(() => { /* log-servern kör inte — tyst fallback */ });
}

// ── React-komponent ───────────────────────────────────────────────────────────

function OpponentSelector({ opponents, grouped, selectedIdx, onChange, accentColor }) {
  const [open, setOpen] = useStateFM(false);
  const [expanded, setExpanded] = useStateFM(() => new Set());
  const ref = useRefFM(null);
  const selected = opponents[selectedIdx];

  useEffFM(() => {
    if (!open) return;
    const onDoc = (e) => { if (ref.current && !ref.current.contains(e.target)) setOpen(false); };
    document.addEventListener('mousedown', onDoc);
    return () => document.removeEventListener('mousedown', onDoc);
  }, [open]);

  const toggle = (v) => {
    const next = new Set(expanded);
    if (next.has(v)) next.delete(v); else next.add(v);
    setExpanded(next);
  };
  const pick = (idx) => { onChange(idx); setOpen(false); };

  return (
    <div ref={ref} style={{position:'relative'}}>
      <button type="button" onClick={()=>setOpen(!open)}
        style={{padding:'10px 14px',fontSize:'15px',width:'100%',textAlign:'left',
          background:'#1f2937',color:'#f3f4f6',border:`1px solid ${accentColor}`,borderRadius:'6px',
          cursor:'pointer',display:'flex',justifyContent:'space-between',alignItems:'center'}}>
        <span>{selected?.label || selected?.name || 'Välj…'}</span>
        <span style={{opacity:0.6,fontSize:'12px'}}>{open ? '▲' : '▼'}</span>
      </button>
      {open && (
        <div style={{position:'absolute',top:'calc(100% + 4px)',left:0,right:0,zIndex:20,
          background:'#1f2937',border:'1px solid #374151',borderRadius:'6px',
          maxHeight:'320px',overflowY:'auto',boxShadow:'0 8px 24px rgba(0,0,0,0.6)'}}>
          {grouped.map((g) => (
            <div key={g.version} style={{borderBottom:'1px solid #374151'}}>
              <div style={{display:'flex',alignItems:'stretch'}}>
                <button type="button"
                  onClick={() => g.champion ? pick(g.champion.idx) : toggle(g.version)}
                  style={{flex:1,padding:'10px 12px',background:selectedIdx===g.champion?.idx?'#374151':'transparent',
                    color:'#f3f4f6',border:'none',textAlign:'left',cursor:'pointer',fontSize:'14px',fontWeight:600}}>
                  {g.champion ? g.champion.label : `${g.version} (ingen champion)`}
                </button>
                {g.others.length > 0 && (
                  <button type="button" onClick={()=>toggle(g.version)}
                    style={{padding:'0 14px',background:'transparent',color:'#9ca3af',
                      border:'none',borderLeft:'1px solid #374151',cursor:'pointer',fontSize:'12px'}}>
                    {expanded.has(g.version) ? '▾' : '▸'} {g.others.length}
                  </button>
                )}
              </div>
              {expanded.has(g.version) && g.others.map((o) => (
                <button key={o.idx} type="button" onClick={()=>pick(o.idx)}
                  style={{display:'block',width:'100%',padding:'8px 12px 8px 28px',
                    background:selectedIdx===o.idx?'#374151':'transparent',
                    color:'#d1d5db',border:'none',textAlign:'left',cursor:'pointer',fontSize:'13px'}}>
                  {o.label || o.name}
                </button>
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function FootballMatch({ matchData, onComplete, onExit }) {
  const canvasRef = useRefFM(null);
  const gRef      = useRefFM(null);
  const keysRef   = useRefFM({});

  const [opponents, setOpponents] = useStateFM([]);
  const [selectedIdx, setSelectedIdx] = useStateFM(0);   // team 0 (your team)
  const [selectedIdx1, setSelectedIdx1] = useStateFM(0); // team 1 (opponent)
  const [started, setStarted] = useStateFM(false);

  // Group opponents by version: { version, champion, others[] }
  const groupedOpponents = useMemoFM(() => {
    const map = new Map();
    opponents.forEach((o, idx) => {
      const v = o.version || 'unknown';
      if (!map.has(v)) map.set(v, { version: v, champion: null, others: [] });
      const slot = map.get(v);
      if (o.name === `${v}-baseline`) slot.champion = { ...o, idx };
      else slot.others.push({ ...o, idx });
    });
    // Sort versions naturally (v1, v2, v3, v4...)
    return Array.from(map.values()).sort((a, b) => {
      const na = parseInt((a.version || '').replace(/\D/g, ''), 10) || 0;
      const nb = parseInt((b.version || '').replace(/\D/g, ''), 10) || 0;
      return na - nb;
    });
  }, [opponents]);

  // Highest version baseline index — used as default selection
  const highestVersionIdx = useMemoFM(() => {
    for (let i = groupedOpponents.length - 1; i >= 0; i--) {
      if (groupedOpponents[i].champion) return groupedOpponents[i].champion.idx;
    }
    return 0;
  }, [groupedOpponents]);

  // Load opponent list on first mount
  useEffFM(() => {
    fetch(`data/policies/opponents.json?t=${Date.now()}`, { cache: 'no-store' })
      .then(r => r.ok ? r.json() : { opponents: [] })
      .then(d => setOpponents((d && d.opponents) || []))
      .catch(() => setOpponents([]));
  }, []);

  // Pre-select highest version when opponents load
  useEffFM(() => {
    if (opponents.length > 0) {
      setSelectedIdx(highestVersionIdx);
      setSelectedIdx1(highestVersionIdx);
    }
  }, [opponents, highestVersionIdx]);

  useEffFM(() => {
    if (!started) return;

    // ── WASM session handles ──────────────────────────────────────────────────
    const wasmHandle = { current: null };
    const humanActive = { current: true };
    const pending = { current: { shoot:false, mega:false, pass:false, passDir:null, tackle:false, jump:false, celebrate:false } };

    // Initialize JS-only state; simulation state filled each frame from WASM.
    gRef.current = {
      teamColors: { ...teamColorsRef.current },
      aiPolicyNames: { 0: 'baseline', 1: 'baseline' },
      _stats: {
        shots:[0,0], passes:[0,0], tackles:[0,0], corners:[0,0],
        possession:[50,50], _possFrames:[0,0],
        freeKicks:[0,0], freeKickGoals:[0,0],
        cornerGoals:[0,0], shotsOnTarget:[0,0],
        _possOwnFrames:[0,0], _possOppFrames:[0,0],
        possOwnHalf:[0,0], possOppHalf:[0,0],
      },
      playerGoals:{}, playerAssists:{}, _log:[],
      _done:false, _lastSetPieceText:null, _activePieceKind:null,
      // Placeholder sim state (safe to render before WASM initialises)
      pl:[], ball:{ x:FW/2,y:h2,vx:0,vy:0,owner:null,mega:false,cooldown:0,lastTouchTeam:null },
      score:[0,0], timer:GAME_SECS*60, phase:'kickoff',
      goalAnim:0, goalTeam:null, setPieceText:'LADDAR...', setPieceTimer:0,
      penaltyTeam:null, penaltyTaken:false, celebration:false, celebrateFrame:0,
      freeKickActive:false, freeKickShooterId:null, gkHasBall:[false,false],
      setPieceTakerId:null, setPieceX:0, setPieceY:0,
    };

    // Fetch team baseline JSON files and initialise WASM session
    const opp0 = opponents[selectedIdx];
    const opp1 = opponents[selectedIdx1];
    const fetchJson = (opp) => opp
      ? fetch(`${opp.file}?t=${Date.now()}`, { cache:'no-store' }).then(r => r.ok ? r.text() : '').catch(() => '')
      : Promise.resolve('');
    let cancelled = false;
    (window._wasmReady || Promise.resolve()).then(() => {
      if (cancelled) return;
      Promise.all([fetchJson(opp0), fetchJson(opp1)]).then(([json0, json1]) => {
        if (cancelled) return;
        const seed = Math.floor(Math.random() * 0xFFFFFFFF);
        wasmHandle.current = wasm_bindgen.create_game(json0, json1, seed);
        const g = gRef.current;
        g.teamColors = { ...teamColorsRef.current };
        g.aiPolicyNames[0] = opp0?.label || opp0?.name || 'baseline';
        g.aiPolicyNames[1] = opp1?.label || opp1?.name || 'baseline';
        const left = (opp0?.label || opp0?.name || 'BASELINE').toUpperCase();
        const right = (opp1?.label || opp1?.name || 'BASELINE').toUpperCase();
        g.setPieceText = `LAG 1: ${left}  vs  LAG 2: ${right}`;
        g.setPieceTimer = 180;
      });
    });
    const canvas = canvasRef.current;
    const ctx    = canvas.getContext('2d');

    function heldPassDirection() {
      const k = keysRef.current;
      let dx=0,dy=0;
      if (k['arrowleft']) dx-=1;
      if (k['arrowright']) dx+=1;
      if (k['arrowup']) dy-=1;
      if (k['arrowdown']) dy+=1;
      if (!dx && !dy) return null;
      const [nx,ny] = norm(dx,dy);
      return { x:nx, y:ny };
    }

    const onKD = (e) => {
      const k=e.key.toLowerCase();
      keysRef.current[k]=true;
      const actionKey = k===' ' || k==='w' || k==='e' || k==='enter' ||
        (keysRef.current['w'] && (k==='arrowleft'||k==='arrowright'||k==='arrowup'||k==='arrowdown'));
      if (e.repeat && actionKey) { e.preventDefault(); return; }
      if (k===' ')     { pending.current.shoot=true; pending.current.mega=!!keysRef.current['q']; pending.current.celebrate=true; e.preventDefault(); }
      if (k==='w')     { pending.current.pass=true; pending.current.passDir=heldPassDirection(); e.preventDefault(); }
      if (keysRef.current['w'] && (k==='arrowleft'||k==='arrowright'||k==='arrowup'||k==='arrowdown')) { pending.current.pass=true; pending.current.passDir=heldPassDirection(); e.preventDefault(); }
      if (k==='e')     { pending.current.tackle=true; e.preventDefault(); }
      if (k==='enter') { pending.current.jump=true; e.preventDefault(); }
      if (k==='backspace') {
        // Toggle human control of player 0. When off, Rust AI controls player 0.
        if (wasmHandle.current !== null) {
          humanActive.current = !humanActive.current;
          wasm_bindgen.set_human_player(wasmHandle.current, humanActive.current);
        }
        e.preventDefault();
      }
      if (k==='escape'){ onExit(); }
    };
    const onKU = (e) => { keysRef.current[e.key.toLowerCase()]=false; };
    window.addEventListener('keydown',onKD);
    window.addEventListener('keyup',onKU);

    function updateWasm() {
      if (wasmHandle.current === null) return;
      const k = keysRef.current;
      const p = pending.current;
      const input = {
        dx: (k['arrowright']||k['d'] ? 1 : 0) - (k['arrowleft']||k['a'] ? 1 : 0),
        dy: (k['arrowdown']||k['s'] ? 1 : 0) - (k['arrowup'] ? 1 : 0),
        shoot: p.shoot,
        mega_shoot: p.mega,
        pass_action: p.pass,
        pass_dir: p.passDir ? [p.passDir.x, p.passDir.y] : null,
        tackle: p.tackle,
        jump: p.jump,
        celebrate: p.celebrate,
      };
      p.shoot=false; p.mega=false; p.pass=false; p.passDir=null; p.tackle=false; p.jump=false; p.celebrate=false;

      const stateJson = wasm_bindgen.step_game(wasmHandle.current, JSON.stringify(input));
      const state = JSON.parse(stateJson);
      const g = gRef.current;

      // Preserve JS-only fields
      const tc=g.teamColors, stats=g._stats, pGoals=g.playerGoals, pAssists=g.playerAssists;
      const log=g._log, aPN=g.aiPolicyNames, lastSPT=g._lastSetPieceText;

      // Merge WASM state into g
      g.pl            = state.pl;
      g.ball          = state.ball;
      g.score         = state.score;
      g.timer         = state.timer;
      g.phase         = state.phase;
      g.goalAnim      = state.goalAnim;
      g.goalTeam      = state.goalTeam;
      g.setPieceText  = state.setPieceText;
      g.setPieceTimer = state.setPieceTimer;
      g.penaltyTeam   = state.penaltyTeam;
      g.penaltyTaken  = state.penaltyTaken;
      g.freeKickActive     = state.freekickActive;
      g.freeKickShooterId  = state.freekickShooterId;
      g.setPieceX     = state.setPieceX;
      g.setPieceY     = state.setPieceY;
      g.gkHasBall     = state.gkHasBall;
      g.celebration   = state.celebration;
      g.celebrateFrame = state.celebrateFrame;
      g._done         = state._done;

      // Override human flag on player 0 based on local toggle
      if (g.pl[0]) g.pl[0].human = humanActive.current;

      // Restore JS-only fields
      g.teamColors=tc; g._stats=stats; g.playerGoals=pGoals; g.playerAssists=pAssists;
      g._log=log; g.aiPolicyNames=aPN; g._lastSetPieceText=lastSPT;

      // SFX events
      const ev = state.events;
      if (ev.goalScored) { try { if (window.SFX) window.SFX.goal(); } catch(e) {} }
      if (ev.shotTaken)  { try { if (window.SFX) window.SFX.shoot(); } catch(e) {} }
      if (ev.passDone)   { try { if (window.SFX) window.SFX.kick(); } catch(e) {} }

      // Accumulate per-team stats from events
      if (ev.shotTaken && g.ball.lastTouchTeam !== null) {
        g._stats.shots[g.ball.lastTouchTeam] = (g._stats.shots[g.ball.lastTouchTeam]||0) + 1;
      }
      if (ev.goalScored && g.goalTeam !== null) {
        g._stats.shotsOnTarget[g.goalTeam] = (g._stats.shotsOnTarget[g.goalTeam]||0) + 1;
      }
      if (ev.passDone && g.ball.lastTouchTeam !== null) {
        g._stats.passes[g.ball.lastTouchTeam] = (g._stats.passes[g.ball.lastTouchTeam]||0) + 1;
      }
      if (ev.tackleDone && g.ball.lastTouchTeam !== null) {
        g._stats.tackles[g.ball.lastTouchTeam] = (g._stats.tackles[g.ball.lastTouchTeam]||0) + 1;
      }
    }


    // Boll-rotation för pentagon-animation
    let _drawFrame = 0;

    function drawFootballBall(ctx, x, y, radius, frame) {
      const rot = frame * 0.018;
      ctx.save();
      ctx.translate(x, y);
      // Skugga
      ctx.fillStyle = 'rgba(0,0,0,0.22)';
      ctx.beginPath(); ctx.ellipse(2, radius + 2, radius * 0.85, radius * 0.28, 0, 0, Math.PI*2); ctx.fill();
      // Clip + vit boll
      ctx.save();
      ctx.beginPath(); ctx.arc(0, 0, radius, 0, Math.PI*2); ctx.clip();
      ctx.fillStyle = '#f8f8f8';
      ctx.beginPath(); ctx.arc(0, 0, radius, 0, Math.PI*2); ctx.fill();
      // Pentagon-mönster
      const drawPent = (cx, cy, r) => {
        ctx.beginPath();
        for (let i = 0; i < 5; i++) {
          const a = (i/5)*Math.PI*2 - Math.PI/2 + rot;
          i===0 ? ctx.moveTo(cx+Math.cos(a)*r, cy+Math.sin(a)*r)
                : ctx.lineTo(cx+Math.cos(a)*r, cy+Math.sin(a)*r);
        }
        ctx.closePath();
        ctx.fillStyle = '#1a1a1a'; ctx.fill();
        ctx.strokeStyle = '#1a1a1a'; ctx.lineWidth = radius * 0.07; ctx.stroke();
      };
      drawPent(0, 0, radius * 0.38);
      for (let i = 0; i < 5; i++) {
        const a = (i/5)*Math.PI*2 - Math.PI/2 + rot;
        const d = radius * 0.68;
        drawPent(Math.cos(a)*d, Math.sin(a)*d, radius * 0.28);
      }
      ctx.restore(); // unclip
      // Yttre outline
      ctx.strokeStyle = '#1a1a1a'; ctx.lineWidth = radius * 0.1;
      ctx.beginPath(); ctx.arc(0, 0, radius, 0, Math.PI*2); ctx.stroke();
      // Highlight
      ctx.fillStyle = 'rgba(255,255,255,0.55)';
      ctx.beginPath(); ctx.ellipse(-radius*0.28, -radius*0.3, radius*0.2, radius*0.12, -0.5, 0, Math.PI*2); ctx.fill();
      ctx.restore();
    }

    // CRT-effekt (scanlines + vignette)
    function drawCRT(ctx, w, h) {
      ctx.fillStyle = 'rgba(0,0,0,0.11)';
      for (let y = 0; y < h; y += 2) ctx.fillRect(0, y, w, 1);
      const vg = ctx.createRadialGradient(w/2,h/2,h*0.28,w/2,h/2,h*0.72);
      vg.addColorStop(0,'rgba(0,0,0,0)');
      vg.addColorStop(1,'rgba(0,0,0,0.32)');
      ctx.fillStyle = vg; ctx.fillRect(0, 0, w, h);
    }

    function draw() {
      const g=gRef.current;
      _drawFrame++;

      // Plan — NES-grönt med alternerande ränder
      const stripeCount = 11;
      const sw = FW / stripeCount;
      for (let i = 0; i < stripeCount; i++) {
        ctx.fillStyle = i % 2 === 0 ? '#1a4a1a' : '#1f5a1f';
        ctx.fillRect(i * sw, 0, sw, FH);
      }
      ctx.strokeStyle='rgba(255,255,255,0.9)'; ctx.lineWidth=2;
      ctx.strokeRect(18,8,FW-36,FH-16);
      ctx.beginPath();ctx.moveTo(FW/2,8);ctx.lineTo(FW/2,FH-8);ctx.stroke();
      ctx.beginPath();ctx.arc(FW/2,h2,62,0,Math.PI*2);ctx.stroke();
      ctx.fillStyle='rgba(255,255,255,0.9)';
      ctx.beginPath();ctx.arc(FW/2,h2,3,0,Math.PI*2);ctx.fill();
      [[18,h2-88,106,176],[FW-124,h2-88,106,176],[18,h2-46,54,92],[FW-72,h2-46,54,92]].forEach(
        ([x,y,w,hh])=>ctx.strokeRect(x,y,w,hh));

      // Mål
      [[FIELD_LINE-GD,h2-GH/2],[FW-FIELD_LINE,h2-GH/2]].forEach(([gx,gy])=>{
        ctx.fillStyle='rgba(255,255,255,0.12)'; ctx.fillRect(gx,gy,GD,GH);
        ctx.strokeStyle='rgba(255,255,255,0.92)'; ctx.lineWidth=3; ctx.strokeRect(gx,gy,GD,GH);
      });

      // Spelare (sorterade på y för djup-ordning)
      [...g.pl].sort((a,b)=>a.y-b.y).forEach(p => drawPlayer(ctx,p,g));

      // Boll — pentagon-mönster (Stil A)
      const b=g.ball;
      if (b.mega) {
        ctx.save(); ctx.translate(b.x,b.y);
        const grd=ctx.createRadialGradient(0,0,BR,0,0,BR*3.8);
        grd.addColorStop(0,'rgba(255,210,40,0.95)');
        grd.addColorStop(1,'rgba(255,80,0,0)');
        ctx.fillStyle=grd; ctx.beginPath();ctx.arc(0,0,BR*3.8,0,Math.PI*2);ctx.fill();
        ctx.restore();
        drawFootballBall(ctx, b.x, b.y, BR*1.5, _drawFrame);
      } else {
        drawFootballBall(ctx, b.x, b.y, BR, _drawFrame);
      }

      // CRT-effekt (Stil A)
      drawCRT(ctx, FW, FH);

      // Kontrolltips längst ned
      const tipBg = ctx.createLinearGradient(0,FH-32,0,FH);
      tipBg.addColorStop(0,'rgba(0,0,0,0)');
      tipBg.addColorStop(0.3,'rgba(0,0,0,0.7)');
      tipBg.addColorStop(1,'rgba(0,0,0,0.85)');
      ctx.fillStyle=tipBg; ctx.fillRect(0,FH-32,FW,32);
      const tips=[
        ['PILAR','rörelse'],['W','passa'],['SPACE','skjut'],
        ['Q+SPC','superskott'],['E','tackling'],['ENTER','hopp'],['BSP','AI']
      ];
      const tipTotalW = tips.length * 110;
      const tipStartX = FW/2 - tipTotalW/2;
      ctx.textBaseline='middle';
      tips.forEach(([key,act],i) => {
        const tx = tipStartX + i*110 + 55;
        ctx.font='bold 8px ui-monospace,monospace';
        const kw = ctx.measureText(key).width + 12;
        ctx.fillStyle='rgba(255,255,255,0.14)';
        const bx = tx - kw/2 - 24;
        ctx.beginPath(); ctx.roundRect(bx, FH-22, kw, 14, 3); ctx.fill();
        ctx.fillStyle='rgba(255,255,255,0.8)';
        ctx.textAlign='center'; ctx.fillText(key, bx + kw/2, FH-15);
        ctx.font='8px ui-monospace,monospace'; ctx.fillStyle='rgba(255,255,255,0.4)';
        ctx.fillText(act, bx + kw/2 + kw/2 + 20, FH-15);
      });

      // Avspark-överlägg (React HUD saknar detta)
      if (g.phase==='kickoff') {
        ctx.fillStyle='rgba(0,0,0,0.62)'; ctx.fillRect(0,0,FW,FH);
        ctx.font='bold 44px Georgia,serif'; ctx.fillStyle='#fff';
        ctx.textAlign='center'; ctx.textBaseline='middle';
        ctx.fillText('AVSPARK',FW/2,h2-22);
        ctx.font='15px ui-monospace,monospace'; ctx.fillStyle='rgba(255,255,255,0.7)';
        ctx.fillText('Tryck valfri tangent',FW/2,h2+18);
        ctx.font='11px ui-monospace,monospace'; ctx.fillStyle='#ffd43b';
        ctx.fillText(`${matchData?.name||'Match'} · Du spelar i BLÅ`,FW/2,h2+50);
      }

      // Fulltime — whistle + crowd fade; summary screen shows via React state
      if (g.phase==='fulltime') {
        if (!g._done) { g._done=true;
          try { if (window.SFX) { window.SFX.whistleFull(); window.SFX.setCrowdTarget(0.06); } } catch(e) {}
        }
      }
    }

    function updateStats() {
      const g = gRef.current;
      if (!g || !g._stats) return;
      if (g.ball.owner !== null) {
        const ownerPlayer = g.pl.find(p => p.id === g.ball.owner);
        const ownerTeam = ownerPlayer?.team;
        if (ownerTeam === 0 || ownerTeam === 1) {
          g._stats._possFrames[ownerTeam]++;
          const total = g._stats._possFrames[0] + g._stats._possFrames[1];
          if (total > 0) {
            g._stats.possession[0] = (g._stats._possFrames[0] / total) * 100;
            g._stats.possession[1] = (g._stats._possFrames[1] / total) * 100;
          }
          // Half possession: team 0 attacks right → own half = left (x < FW/2)
          const inOwnHalf = ownerTeam === 0 ? g.ball.x < FW/2 : g.ball.x >= FW/2;
          if (inOwnHalf) {
            g._stats._possOwnFrames[ownerTeam]++;
          } else {
            g._stats._possOppFrames[ownerTeam]++;
          }
          const totalOwn = g._stats._possOwnFrames[0] + g._stats._possOwnFrames[1];
          const totalOpp = g._stats._possOppFrames[0] + g._stats._possOppFrames[1];
          const totalAll = totalOwn + totalOpp;
          if (totalAll > 0) {
            g._stats.possOwnHalf[0] = g._stats._possOwnFrames[0] / totalAll * 100;
            g._stats.possOwnHalf[1] = g._stats._possOwnFrames[1] / totalAll * 100;
            g._stats.possOppHalf[0] = g._stats._possOppFrames[0] / totalAll * 100;
            g._stats.possOppHalf[1] = g._stats._possOppFrames[1] / totalAll * 100;
          }
        }
      }
      if (window.SFX) {
        try {
          const b = g.ball;
          const nearLeft  = b.x < FW * 0.2;
          const nearRight = b.x > FW * 0.8;
          const tension = (nearLeft || nearRight)
            ? clamp(1 - (nearLeft ? b.x : FW - b.x) / (FW * 0.2), 0, 1)
            : 0;
          window.SFX.setCrowdTension(tension);
        } catch(e) {}
      }
      if (g._lastSetPieceText !== g.setPieceText && g.setPieceText &&
          (g.setPieceText.includes('FRISPARK') || g.setPieceText.includes('STRAFF') || g.setPieceText.includes('HÖRNA'))) {
        try { if (window.SFX) window.SFX.whistle(); } catch(e) {}
      }
      g._lastSetPieceText = g.setPieceText;
    }

    let raf;
    const loop=()=>{ updateWasm(); updateStats(); draw(); raf=requestAnimationFrame(loop); };
    raf=requestAnimationFrame(loop);
    return ()=>{
      cancelled=true;
      cancelAnimationFrame(raf);
      window.removeEventListener('keydown',onKD);
      window.removeEventListener('keyup',onKU);
      if (wasmHandle.current !== null) { try { wasm_bindgen.destroy_game(wasmHandle.current); } catch(e) {} }
      try { if (window.SFX) window.SFX.stopAll(); } catch(e) {}
    };
  },[started]);

  const [matchTeam0, setMatchTeam0] = useStateFM(null);
  const [matchTeam1, setMatchTeam1] = useStateFM(null);
  const [matchRoster0, setMatchRoster0] = useStateFM(null);
  const [matchRoster1, setMatchRoster1] = useStateFM(null);
  const [gameSnapshot, setGameSnapshot] = useStateFM(null);
  const [matchStats, setMatchStats] = useStateFM(null);
  const [finalSummary, setFinalSummary] = useStateFM(null);
  const [tsReady, setTsReady] = useStateFM(() => !!window.TeamSelectScreen);

  useEffFM(() => {
    if (tsReady) return;
    const id = setInterval(() => {
      if (window.TeamSelectScreen) { setTsReady(true); clearInterval(id); }
    }, 50);
    return () => clearInterval(id);
  }, [tsReady]);

  useEffFM(() => {
    const unlock = () => { try { if (window.SFX) window.SFX.unlock(); } catch(e) {} };
    window.addEventListener('keydown', unlock, { once: true });
    window.addEventListener('pointerdown', unlock, { once: true });
    return () => {
      window.removeEventListener('keydown', unlock);
      window.removeEventListener('pointerdown', unlock);
    };
  }, []);

  useEffFM(() => {
    if (!started) return;
    const id = setInterval(() => {
      const g = gRef.current;
      if (!g) return;
      setGameSnapshot({
        score: [...g.score], timer: g.timer,
        phase: g.phase, setPieceText: g.setPieceText,
        setPieceTimer: g.setPieceTimer
      });
      if (g._stats) setMatchStats({
        shots: [...g._stats.shots], passes: [...g._stats.passes],
        tackles: [...g._stats.tackles], corners: [...g._stats.corners],
        possession: [...g._stats.possession],
        freeKicks: [...g._stats.freeKicks], freeKickGoals: [...g._stats.freeKickGoals],
        cornerGoals: [...g._stats.cornerGoals], shotsOnTarget: [...g._stats.shotsOnTarget],
        possOwnHalf: [...g._stats.possOwnHalf], possOppHalf: [...g._stats.possOppHalf],
      });
      if (g.phase === 'fulltime') {
        setFinalSummary(prev => {
          if (prev) return prev;
          const summary = {
            score: [...g.score],
            team0: teamNamesRef.current.t0,
            team1: teamNamesRef.current.t1,
            stats: {
              shots: [...g._stats.shots], shotsOnTarget: [...g._stats.shotsOnTarget],
              corners: [...g._stats.corners], cornerGoals: [...g._stats.cornerGoals],
              freeKicks: [...g._stats.freeKicks], freeKickGoals: [...g._stats.freeKickGoals],
              possession: [...g._stats.possession],
              possOwnHalf: [...g._stats.possOwnHalf], possOppHalf: [...g._stats.possOppHalf],
            },
            log: [...g._log],
            playerGoals: { ...g.playerGoals },
            playerAssists: { ...g.playerAssists },
          };
          saveMatchLog(summary);
          return summary;
        });
      }
    }, 120);
    return () => clearInterval(id);
  }, [started]);

  const teamColorsRef = useRefFM({ 0: '#3464a8', 1: '#c82828' });
  const teamNamesRef = useRefFM({ t0: 'lag0', t1: 'lag1' });

  const handleTeamSelectStart = ({ team0, team1, oppIdx0, oppIdx1, roster0, roster1 }) => {
    setMatchTeam0(team0);
    setMatchTeam1(team1);
    setMatchRoster0(roster0 || null);
    setMatchRoster1(roster1 || null);
    teamNamesRef.current = { t0: team0?.name || 'lag0', t1: team1?.name || 'lag1' };
    teamColorsRef.current = {
      0: team0?.primary || '#3464a8',
      1: team1?.primary || '#c82828',
    };
    setSelectedIdx(oppIdx0);
    setSelectedIdx1(oppIdx1);
    setStarted(true);
  };

  return (
    <div style={{position:'absolute',inset:0,background:'#0a0a0a',
      display:'flex',alignItems:'center',justifyContent:'center',overflow:'hidden'}}>

      {!started && tsReady && (
        <window.TeamSelectScreen
          opponents={opponents}
          groupedOpponents={groupedOpponents}
          highestVersionIdx={highestVersionIdx}
          onStart={handleTeamSelectStart}
          onExit={onExit}
        />
      )}

      {!started && !tsReady && (
        <div style={{color:'rgba(255,255,255,0.25)',fontFamily:'ui-monospace,monospace',fontSize:11,letterSpacing:'0.15em'}}>
          LADDAR…
        </div>
      )}

      <canvas ref={canvasRef} width={FW} height={FH}
        style={{maxWidth:'100%',maxHeight:'100%',display:'block',
          visibility: started ? 'visible' : 'hidden'}}/>

      {started && window.MatchHUD && !finalSummary && (
        <window.MatchHUD
          game={gameSnapshot}
          team0info={matchTeam0}
          team1info={matchTeam1}
          matchStats={matchStats}
          onExit={onExit}
        />
      )}

      {finalSummary && window.MatchSummary && (
        <window.MatchSummary
          score={finalSummary.score}
          stats={finalSummary.stats}
          log={finalSummary.log}
          playerGoals={finalSummary.playerGoals}
          playerAssists={finalSummary.playerAssists}
          team0={matchTeam0}
          team1={matchTeam1}
          roster0={matchRoster0}
          roster1={matchRoster1}
          onContinue={() => onComplete && onComplete(
            finalSummary.score[0] > finalSummary.score[1], matchData?.id
          )}
          onExit={onExit}
        />
      )}
    </div>
  );
}

window.FootballMatch = FootballMatch;
