const { useEffect: useEffFM, useRef: useRefFM, useState: useStateFM, useMemo: useMemoFM } = React;

// ── Konstanter ───────────────────────────────────────────────────────────────
const FW = 880, FH = 520, GH = 130, GD = 26;
const PR = 14, BR = 8;
const PSPEED = 3.5, CSPEED = 2.4;
const BALL_FRIC = 0.968;
const PASS_POW = 11, SHOOT_POW = 16, MEGA_POW = 27, CPU_PASS_POW = 10;
const MEGA_KR = 55;
const KNOCK_DUR = 300;
const BALL_COOL = 16;
const TACKLE_DIST = 34;
const TACKLE_COOL = 70;
const TACKLE_MISS_DUR = 30;
const JUMP_DUR = 28;
const PASS_BLOCK_DIST = 25;
const PENALTY_AREA_W = 124;
const PENALTY_SPOT_D = 132;
const GAME_SECS = 150;
const h2 = FH / 2;

// Set pieces & new mechanics
const SLOW_DUR = 6;
const SLOW_FACTOR = 0.5;
const FOUL_PAUSE = 30;
const FREE_KICK_WALL_DIST = 55;
const GK_DIVE_DUR = 6;
const GK_DIVE_COMMIT_DIST = 160;
const GK_DIVE_JITTER = 40;
const GK_HOLD_DELAY = 60;
const SET_PIECE_DELAY = 60;
const FIELD_LINE = 18;
const GOAL_AREA_W = 54;
const TACKLE_BALL_NUDGE_POW = 6;

const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
const norm  = (dx, dy) => { const m = Math.hypot(dx, dy) || 1; return [dx/m, dy/m]; };

const BASELINE_AI_PARAMS = {
  passChancePressured: 0.16,
  passChanceWing: 0.07,
  passChanceForward: 0.04,
  passChanceDefault: 0.055,
  shootProgressThreshold: 0.76,
  tackleChance: 0.08,
  forwardPassMinGain: 8,
  markDistance: 48,
};

const HAIR = ['#5a3a1a','#1a1a1a','#f4d090','#8b0000','#ffd700','#5a3a1a',
              '#1a1a1a','#f4d090','#5a3a1a','#c87850'];

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
  const baseColors = { 0: NES_TEAM0, 1: NES_TEAM1 };
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

// ── Spelstatus ────────────────────────────────────────────────────────────────

function mkP(id, team, x, y, role, human=false) {
  return { id, team, x, y, vx:0, vy:0, role, human, state:'active',
           knockTimer:0, homeX:x, homeY:y, facing:'down',
           stepCounter:0, hairColor: HAIR[id], celebrateTimer:0, tackleCooldown:0, jumpTimer:0,
           aiJitterX:0, aiJitterY:0, aiJitterTimer:0,
           slowTimer:0, gkDiveDir:null, gkDiveTimer:0, gkHoldTimer:0 };
}

function newGame() {
  return {
    pl: [
      mkP(0,0,FW*.44,h2,    'fwd',true),
      mkP(1,0,FW*.32,h2-85, 'mid'),
      mkP(2,0,FW*.32,h2+85, 'mid'),
      mkP(3,0,FW*.17,h2,    'def'),
      mkP(4,0,FIELD_LINE+PR*2,h2, 'gk'),
      mkP(5,1,FW*.56,h2,    'fwd'),
      mkP(6,1,FW*.68,h2-85, 'mid'),
      mkP(7,1,FW*.68,h2+85, 'mid'),
      mkP(8,1,FW*.83,h2,    'def'),
      mkP(9,1,FW-FIELD_LINE-PR*2,h2, 'gk'),
    ],
    ball:{ x:FW/2,y:h2,vx:0,vy:0,owner:null,mega:false,cooldown:0,lastTouchTeam:null },
    score:[0,0], timer:GAME_SECS*60,
    phase:'kickoff', goalAnim:0, goalTeam:null,
    setPieceText:null, setPieceTimer:0, penaltyTeam:null, penaltyTaken:false,
    aiPolicies:{ 0: BASELINE_AI_PARAMS, 1: BASELINE_AI_PARAMS },
    aiPolicyNames:{ 0: 'baseline', 1: 'baseline' },
    lastScorer:null, celebration:false, celebrateFrame:0,
    freeKickActive:false, freeKickShooterId:null,
    gkHasBall:[false,false],
    setPieceTakerId:null, setPieceX:0, setPieceY:0,
    _done:false,
    _stats: { shots:[0,0], passes:[0,0], tackles:[0,0], corners:[0,0], possession:[50,50], _possFrames:[0,0] },
  };
}

function doShoot(g, shooter, mega, tx, ty, pow) {
  const p = pow || (mega ? MEGA_POW : SHOOT_POW);
  const ball = g.ball;
  const [nx,ny] = norm(tx - shooter.x, ty - shooter.y);
  ball.vx=nx*p; ball.vy=ny*p;
  ball.x=shooter.x; ball.y=shooter.y;
  ball.owner=null; ball.mega=mega; ball.cooldown=BALL_COOL;
  ball.lastTouchTeam=shooter.team;
  try {
    if (window.SFX) {
      const isShot = p >= SHOOT_POW;
      if (mega) window.SFX.shoot();
      else if (isShot) window.SFX.shoot();
      else window.SFX.kick();
    }
  } catch(e) {}
  if (g._stats && p >= SHOOT_POW) {
    const t = shooter.team;
    const towardsGoal = t===0 ? nx > 0.4 : nx < -0.4;
    if (towardsGoal) g._stats.shots[t] = (g._stats.shots[t]||0) + 1;
  } else if (g._stats) {
    const t = shooter.team;
    g._stats.passes[t] = (g._stats.passes[t]||0) + 1;
  }
}

function knockPlayer(g, p, duration=KNOCK_DUR) {
  if (!p || p.state !== 'active') return;
  p.state = 'knocked';
  p.knockTimer = duration;
  p.jumpTimer = 0;
  if (g.ball.owner === p.id) {
    g.ball.owner = null;
    g.ball.x = p.x;
    g.ball.y = p.y;
    g.ball.vx = 0;
    g.ball.vy = 0;
    g.ball.mega = false;
    g.ball.cooldown = BALL_COOL;
  }
}

function isJumping(p) {
  return p && p.state==='active' && p.jumpTimer>0;
}

function tacklePlayer(g, tackler, target) {
  if (!tackler || !target || tackler.state!=='active' || target.state!=='active') return false;
  tackler.tackleCooldown = TACKLE_COOL;

  const targetHasBall = g.ball.owner === target.id;

  if (isJumping(target)) {
    knockPlayer(g, tackler, TACKLE_MISS_DUR);
    return true;
  }

  if (targetHasBall) {
    // On-ball tackle: strip ball, nudge it forward in tackler's direction
    const b = g.ball;
    const [nx, ny] = norm(target.x - tackler.x, target.y - tackler.y);
    b.owner = null;
    b.vx = nx * TACKLE_BALL_NUDGE_POW;
    b.vy = ny * TACKLE_BALL_NUDGE_POW;
    b.x = target.x; b.y = target.y;
    b.cooldown = BALL_COOL;
    slowPlayer(target, SLOW_DUR);
    try { if (window.SFX) window.SFX.tackle(); } catch(e) {}
    if (g._stats) g._stats.tackles[tackler.team] = (g._stats.tackles[tackler.team]||0) + 1;
  } else {
    // Off-ball tackle: foul. No pause/knock — slow target briefly, free kick.
    if (target.team!==tackler.team && isInOwnPenaltyArea(tackler)) {
      startPenalty(g, target.team);
      return true;
    }
    slowPlayer(target, SLOW_DUR * 4);
    startFreeKick(g, target, target.x, target.y);
  }
  return true;
}

function moveTo(p, tx, ty, speed) {
  const [nx,ny] = norm(tx-p.x, ty-p.y);
  const npx = p.x + nx*speed, npy = p.y + ny*speed;
  p.x=clamp(npx,PR,FW-PR); p.y=clamp(npy,PR,FH-PR);
  // Facing + step
  if (Math.hypot(nx,ny) > 0.1) {
    p.facing = Math.abs(nx)>Math.abs(ny) ? (nx>0?'right':'left') : (ny>0?'down':'up');
    p.stepCounter++;
  }
}

function teamPlayers(g, team) {
  return g.pl.filter(p => p.team===team && p.state==='active');
}

function rolePlayer(g, team, role) {
  return teamPlayers(g, team).find(p => p.role===role) || teamPlayers(g, team)[0];
}

// Returns a player's effective policy: their per-player override if set,
// otherwise the team-level policy from g.aiPolicies.
function effectivePolicy(g, p) {
  return p.aiPolicy || g.aiPolicies?.[p.team] || BASELINE_AI_PARAMS;
}

// Detects v1/v2/v3 format from opponent metadata + policy file shape, then
// installs brains on every player on both teams (we play the user's team
// using the same opponent policy too, like before).
// Apply a policy ONLY to a specific team (0 or 1). Lets each team have its
// own trained model. Returns the resolved name for HUD display.
function applyPolicyToTeam(g, opp, policy, team) {
  const ver = (opp.version || policy.type || '').toString();
  const pp = Array.isArray(policy.playerParams) ? policy.playerParams : null;
  const hasV3Wrapper = pp && pp[0] && pp[0].v3;
  const hasBase = pp && pp[0] && pp[0].base;
  const matches = (p) => p.team === team;

  const hasV6Spatial = pp && pp[0] && pp[0].spatial && pp[0].decisions;
  if (hasV6Spatial) {
    g.pl.forEach(p => {
      if (!matches(p)) return;
      const slot = p.id % 5;
      p.brain = { version: 'v6', params: pp[slot] || pp[0] || {} };
    });
    return policy.name || opp.name || 'v6';
  }
  if (hasV3Wrapper) {
    g.pl.forEach(p => {
      if (!matches(p)) return;
      const slot = p.id % 5;
      p.brain = { version: 'v4', params: pp[slot] || pp[0] || {} };
    });
    return policy.name || opp.name || 'v4';
  }
  if (hasBase || (ver === 'v3' && pp)) {
    g.pl.forEach(p => {
      if (!matches(p)) return;
      const slot = p.id % 5;
      p.brain = { version: 'v3', params: pp[slot] || pp[0] || {} };
    });
    return policy.name || opp.name || 'v3';
  }
  if (Array.isArray(pp)) {
    g.pl.forEach(p => {
      if (!matches(p)) return;
      const slot = p.id % 5;
      const params = { ...BASELINE_AI_PARAMS, ...(pp[slot] || pp[0] || {}) };
      p.brain = { version: 'v2', params };
    });
    return policy.name || opp.name || 'v2';
  }
  // v1 fallback
  const params = { ...BASELINE_AI_PARAMS, ...(policy.parameters || {}) };
  g.pl.forEach(p => { if (matches(p)) p.brain = { version: 'v1', params }; });
  g.aiPolicies[team] = params;
  return policy.name || opp.name || 'candidate';
}

// Backward-compatible wrapper: apply same opp+policy to BOTH teams.
function applyOpponentPolicy(g, opp, policy) {
  const label = (opp.label || policy.name || opp.name || 'CANDIDATE').toUpperCase();
  applyPolicyToTeam(g, opp, policy, 0);
  const name = applyPolicyToTeam(g, opp, policy, 1);
  g.aiPolicyNames[1] = name;
  g.setPieceText = `MOTSTÅNDARE: ${label}`;
  g.setPieceTimer = 120;
}

function setBallOwner(g, p, x, y, text) {
  const b = g.ball;
  p.x = clamp(x, PR, FW-PR);
  p.y = clamp(y, PR, FH-PR);
  p.state='active'; p.knockTimer=0; p.jumpTimer=0;
  b.x=p.x; b.y=p.y; b.vx=0; b.vy=0; b.owner=p.id; b.mega=false; b.cooldown=BALL_COOL;
  b.lastTouchTeam=p.team;
  g.phase='playing';
  g.setPieceText=text;
  g.setPieceTimer=90;
  g.setPieceTakerId=null;
}

// Place ball at set-piece spot, mark a specific taker. Match continues normally.
function awardSetPiece(g, takerId, sx, sy, text) {
  const b = g.ball;
  b.x = sx; b.y = sy; b.vx = 0; b.vy = 0;
  b.owner = null; b.mega = false; b.cooldown = 0;
  g.setPieceTakerId = takerId;
  g.setPieceX = sx; g.setPieceY = sy;
  g.setPieceText = text;
  g.setPieceTimer = 120;
  g.phase = 'playing';
}

function slowPlayer(p, dur) {
  p.slowTimer = dur || SLOW_DUR;
}

function goalLineTeams(x) {
  return x < FW/2
    ? { attacking:1, defending:0 }
    : { attacking:0, defending:1 };
}

function isInPenaltyAreaForTeam(team, x, y) {
  const inY = Math.abs(y-h2) <= GH/2 + 38;
  if (!inY) return false;
  return team===0 ? x <= PENALTY_AREA_W : x >= FW-PENALTY_AREA_W;
}

function isInOwnPenaltyArea(p) {
  return isInPenaltyAreaForTeam(p.team, p.x, p.y);
}

function startFreeKick(g, fouledPlayer, fx, fy) {
  awardSetPiece(g, fouledPlayer.id, fx, fy, 'FRISPARK');
  g.freeKickShooterId = fouledPlayer.id;
  g.freeKickActive = true;
}

function restartGoalKick(g, team) {
  const keeper = rolePlayer(g, team, 'gk');
  const sx = team===0 ? FIELD_LINE + PR*2.3 : FW - FIELD_LINE - PR*2.3;
  awardSetPiece(g, keeper.id, sx, h2, 'MÅLVAKTENS BOLL');
  g.gkHasBall[team] = false;
}

function restartKickIn(g, team, x, y) {
  const taker = teamPlayers(g, team)
    .filter(p => p.role!=='gk')
    .sort((a,b)=>Math.hypot(a.x-x,a.y-y)-Math.hypot(b.x-x,b.y-y))[0] || rolePlayer(g, team, 'mid');
  const sy = y < h2 ? PR : FH-PR;
  awardSetPiece(g, taker.id, clamp(x, PR, FW-PR), sy, 'INSPARK');
}

function restartCorner(g, team, x, y) {
  const taker = rolePlayer(g, team, 'mid') || rolePlayer(g, team, 'fwd');
  const sx = x < FW/2 ? FIELD_LINE + PR : FW - FIELD_LINE - PR;
  const sy = y < h2 ? PR : FH-PR;
  awardSetPiece(g, taker.id, sx, sy, 'HÖRNA');
  try { if (window.SFX) window.SFX.corner(); } catch(e) {}
  if (g._stats) g._stats.corners[team] = (g._stats.corners[team]||0) + 1;
}

function startPenalty(g, team) {
  const shooter = team===0 ? g.pl[0] : (rolePlayer(g, team, 'fwd') || rolePlayer(g, team, 'mid'));
  const sx = team===0 ? FW - FIELD_LINE - PENALTY_SPOT_D : FIELD_LINE + PENALTY_SPOT_D;
  g.pl.forEach(p => {
    p.state='active'; p.knockTimer=0; p.jumpTimer=0; p.tackleCooldown=Math.max(p.tackleCooldown, FOUL_PAUSE + SET_PIECE_DELAY);
    if (p.id === shooter.id) return;
    const oppGk = p.role==='gk' && p.team!==team;
    if (!oppGk) {
      if (team===0 && p.x > FW - FIELD_LINE - PENALTY_AREA_W - 20) {
        p.x = FW - FIELD_LINE - PENALTY_AREA_W - 20 - Math.random()*40;
      } else if (team===1 && p.x < FIELD_LINE + PENALTY_AREA_W + 20) {
        p.x = FIELD_LINE + PENALTY_AREA_W + 20 + Math.random()*40;
      }
    }
  });
  setBallOwner(g, shooter, sx, h2, 'STRAFF');
  g.phase='penalty';
  g.penaltyTeam=team;
  g.penaltyTaken=false;
}

function handleBallOut(g) {
  const b = g.ball;
  if (b.y-BR <= 0 || b.y+BR >= FH) {
    const restartTeam = b.lastTouchTeam===0 ? 1 : 0;
    restartKickIn(g, restartTeam, b.x, b.y);
    return true;
  }

  if (b.x-BR <= FIELD_LINE || b.x+BR >= FW-FIELD_LINE) {
    const { attacking, defending } = goalLineTeams(b.x);
    if (b.lastTouchTeam===attacking) restartGoalKick(g, defending);
    else restartCorner(g, attacking, b.x, b.y);
    return true;
  }
  return false;
}

// ── Förbättrad AI ─────────────────────────────────────────────────────────────

function isMarked(g, p, threshold=55) {
  return g.pl.some(q => q.team!==p.team && q.state==='active' && Math.hypot(q.x-p.x,q.y-p.y)<threshold);
}

function ownGoalPoint(team) {
  return { x: team===0 ? 0 : FW, y: h2 };
}

function oppGoalPoint(team) {
  return { x: team===0 ? FW+GD : -GD, y: h2 };
}

function teamDir(team) {
  return team===0 ? 1 : -1;
}

function attackProgress(team, x) {
  return team===0 ? x/FW : 1 - x/FW;
}

function sideOf(p) {
  return p.homeY < h2 ? -1 : 1;
}

function wingY(p) {
  return sideOf(p) < 0 ? 58 : FH-58;
}

function pointBetween(a, b, t) {
  return { x:a.x+(b.x-a.x)*t, y:a.y+(b.y-a.y)*t };
}

function distToSegment(px, py, ax, ay, bx, by) {
  const vx=bx-ax, vy=by-ay;
  const len2=vx*vx+vy*vy || 1;
  const t=clamp(((px-ax)*vx+(py-ay)*vy)/len2,0,1);
  const sx=ax+vx*t, sy=ay+vy*t;
  return Math.hypot(px-sx, py-sy);
}

function passLineOpen(g, from, to, team, blockDist=PASS_BLOCK_DIST) {
  return !g.pl.some(q => {
    if (q.team===team || q.state!=='active') return false;
    return distToSegment(q.x,q.y,from.x,from.y,to.x,to.y) < blockDist;
  });
}

function nearestOpponentDistance(g, p) {
  let best = Infinity;
  g.pl.forEach(q => {
    if (q.team===p.team || q.state!=='active') return;
    best = Math.min(best, Math.hypot(q.x-p.x,q.y-p.y));
  });
  return best;
}

function passMovesForward(from, to, minGain=8) {
  return (to.x-from.x) * teamDir(from.team) > minGain;
}

function looseBallChaser(g) {
  let best = null, bestD = Infinity;
  g.pl.forEach(p => {
    if (p.human || p.role==='gk' || p.state!=='active') return;
    const d = Math.hypot(p.x-g.ball.x,p.y-g.ball.y);
    if (d < bestD) { bestD=d; best=p; }
  });
  return best;
}

function getLooseBallSupportTarget(g, p) {
  const ball = g.ball;
  if (p.role==='fwd') {
    return {
      x: p.team===0 ? Math.max(shapeXWithBall(p, ball.x, 0.58), FW*0.58) : Math.min(shapeXWithBall(p, ball.x, 0.58), FW*0.42),
      y: h2 + (p.homeY-h2)*0.45,
    };
  }
  if (p.role==='mid') {
    return {
      x: shapeXWithBall(p, ball.x, 0.7),
      y: h2 + (p.homeY-h2)*0.78 + (ball.y-h2)*0.16,
    };
  }
  if (p.role==='def') {
    return {
      x: p.team===0 ? Math.min(shapeXWithBall(p, ball.x, 0.42), FW*0.43) : Math.max(shapeXWithBall(p, ball.x, 0.42), FW*0.57),
      y: h2 + (ball.y-h2)*0.18,
    };
  }
  return { x:p.homeX, y:p.homeY };
}

function nearestOpponentCarrier(g, team) {
  const carrier = g.pl.find(q => q.id===g.ball.owner);
  if (carrier && carrier.team!==team) return carrier;
  return null;
}

function defensiveBlockTarget(g, p, carrier) {
  const goal = ownGoalPoint(p.team);
  if (!carrier) return { x:p.homeX, y:p.homeY };
  const t = p.role==='def' ? 0.38 : 0.55;
  return pointBetween(goal, carrier, t);
}

function bestInterceptionTarget(g, p, carrier) {
  let best = null, bestScore = -Infinity;
  g.pl.forEach(opp => {
    if (opp.team===p.team || opp.id===carrier?.id || opp.state!=='active') return;
    const d = distToSegment(p.x,p.y,carrier.x,carrier.y,opp.x,opp.y);
    const passLaneD = distToSegment(opp.x,opp.y,carrier.x,carrier.y,p.x,p.y);
    const score = -d - passLaneD*0.35 + (opp.role==='fwd'?35:0);
    if (score > bestScore) {
      bestScore = score;
      best = pointBetween(carrier, opp, 0.48);
    }
  });
  return best;
}

function naturalTarget(p, target, amp=16) {
  p.aiJitterTimer--;
  if (p.aiJitterTimer<=0) {
    p.aiJitterX = (Math.random()*2-1)*amp;
    p.aiJitterY = (Math.random()*2-1)*amp;
    p.aiJitterTimer = 35 + Math.floor(Math.random()*55);
  }
  return {
    x: clamp(target.x+p.aiJitterX, PR, FW-PR),
    y: clamp(target.y+p.aiJitterY, PR, FH-PR),
  };
}

function shapeXWithBall(p, ballX, strength=0.55) {
  return clamp(p.homeX + (ballX - FW/2)*strength, PR, FW-PR);
}

function getAttackTarget(g, p) {
  const carrier = g.pl.find(q => q.id===g.ball.owner);
  if (!carrier) return { x:p.homeX, y:p.homeY };
  const ball = g.ball;

  if (p.role==='fwd') {
    const baseX = shapeXWithBall(p, ball.x, 0.62);
    const runX = p.team===0 ? Math.max(baseX, FW*0.62) : Math.min(baseX, FW*0.38);
    const openY = passLineOpen(g, carrier, {x:runX,y:h2-42}, p.team) ? h2-42 : h2+42;
    return { x:runX, y:openY };
  }
  if (p.role==='mid') {
    const dir = teamDir(p.team);
    const followX = shapeXWithBall(p, ball.x, 0.72);
    const supportX = carrier.id===p.id ? p.x : carrier.x + dir*76;
    const laneX = clamp((followX+supportX)/2, FW*0.16, FW*0.84);
    return { x:laneX, y:wingY(p) };
  }
  if (p.role==='def') {
    const x = p.team===0 ? Math.min(shapeXWithBall(p, ball.x, 0.38), FW*0.43) : Math.max(shapeXWithBall(p, ball.x, 0.38), FW*0.57);
    return { x, y:h2 };
  }
  return { x:p.homeX, y:p.homeY };
}

function getDefendTarget(g, p) {
  const ball = g.ball;
  const carrier = g.pl.find(q => q.id===ball.owner);
  const opponentCarrier = carrier && carrier.team!==p.team ? carrier : null;
  const ownGoalX = p.team===0 ? 0 : FW;
  if (p.role==='fwd') {
    return opponentCarrier ? { x:opponentCarrier.x, y:opponentCarrier.y } : { x:ball.x, y:ball.y };
  }
  if (p.role==='def') return defensiveBlockTarget(g,p,opponentCarrier || {x:ball.x,y:ball.y});
  if (p.role==='mid' && opponentCarrier) {
    if (Math.hypot(p.x-opponentCarrier.x,p.y-opponentCarrier.y)<190) {
      const block = defensiveBlockTarget(g,p,opponentCarrier);
      const intercept = bestInterceptionTarget(g,p,opponentCarrier);
      return intercept && Math.random()<0.45 ? intercept : block;
    }
  }
  const ratio = p.role==='mid' ? 0.64 : 0.22;
  const bx = ownGoalX + (ball.x-ownGoalX)*ratio;
  const by = h2 + (ball.y-h2)*0.38 + (p.homeY-h2)*0.34;
  return { x:bx, y:clamp(by,PR,FH-PR) };
}

function cpuFindPass(g, carrier) {
  const params = effectivePolicy(g, carrier);
  const oppGoalX = carrier.team===0 ? FW : 0;
  let best=null, bestScore=-Infinity;
  g.pl.forEach(p => {
    if (p.team!==carrier.team||p.id===carrier.id||p.state!=='active') return;
    if (isMarked(g,p,params.markDistance ?? 48)) return;
    if (!passLineOpen(g, carrier, p, carrier.team)) return;
    const forwardGain = (p.x-carrier.x) * teamDir(carrier.team);
    const gain = Math.abs(carrier.x-oppGoalX) - Math.abs(p.x-oppGoalX);
    const width = Math.abs(p.y-h2);
    const inFrontOfGoal = p.role==='fwd' ? 85 : 0;
    const wingBonus = p.role==='mid' ? 70 + width*0.35 : 0;
    const cutbackBonus = carrier.role==='mid' && ((carrier.team===0 && carrier.x>FW*0.50) || (carrier.team===1 && carrier.x<FW*0.50)) ? inFrontOfGoal + (Math.abs(p.y-h2)<75 ? 55 : 0) : 0;
    const centralCarrierBonus = Math.abs(carrier.y-h2)<55 && p.role==='mid' ? 110 : 0;
    const minGain = params.forwardPassMinGain ?? 8;
    const forwardBonus = forwardGain > minGain ? 150 + forwardGain*1.15 : forwardGain*8;
    const score = gain + forwardBonus + wingBonus + cutbackBonus + centralCarrierBonus - Math.hypot(p.x-carrier.x,p.y-carrier.y)*0.05;
    if (score>bestScore) { bestScore=score; best=p; }
  });
  return best;
}

// Brain dispatcher. v1/v2 use the classic cpuTick; v3 wraps it with
// modulators; v4 = v3 + directional pass mult + GK freedom.
function tickPlayer(g, p) {
  if (!p.brain) { cpuTick(g, p); return; }
  switch (p.brain.version) {
    case 'v6': v6Tick(g, p); return;
    case 'v4': v4Tick(g, p); return;
    case 'v3': v3Tick(g, p); return;
    case 'v1':
    case 'v2':
    default: {
      const saved = p.aiPolicy;
      p.aiPolicy = p.brain.params;
      cpuTick(g, p);
      p.aiPolicy = saved;
      return;
    }
  }
}

// V6 distance-pref cost: quadratic around preferred + soft penalty outside [min,max]
function v6PrefCost(d, pref) {
  if (!pref) return 0;
  const range = Math.max(1, pref.max - pref.min);
  const norm = (d - pref.preferred) / range;
  const base = norm * norm;
  const below = Math.max(0, pref.min - d);
  const above = Math.max(0, d - pref.max);
  return base + below*below*0.01 + above*above*0.01;
}

function v6NearestActive(g, excludeId, wantTeam, x, y) {
  let best = Infinity;
  for (const q of g.pl) {
    if (q.state !== 'active' || q.id === excludeId) continue;
    if (wantTeam !== null && q.team !== wantTeam) continue;
    const d = Math.hypot(q.x - x, q.y - y);
    if (d < best) best = d;
  }
  return Number.isFinite(best) ? best : 600;
}

function v6CostAt(g, p, x, y, sp) {
  const ownGx = p.team === 0 ? FIELD_LINE : FW - FIELD_LINE;
  const dGoal = Math.hypot(x - ownGx, y - h2);
  const dSide = y;
  const dBall = Math.hypot(x - g.ball.x, y - g.ball.y);
  const dTeam = v6NearestActive(g, p.id, p.team, x, y);
  const dOpp  = v6NearestActive(g, p.id, 1 - p.team, x, y);
  return v6PrefCost(dGoal, sp.ownGoal)
       + v6PrefCost(dSide, sp.side)
       + v6PrefCost(dBall, sp.ball)
       + v6PrefCost(dTeam, sp.teammate)
       + v6PrefCost(dOpp,  sp.opponent);
}

function v6Target(g, p, sp) {
  const r = 24, s = r * 0.7071;
  const cands = [
    [p.x, p.y],
    [p.x+r, p.y], [p.x-r, p.y], [p.x, p.y+r], [p.x, p.y-r],
    [p.x+s, p.y+s], [p.x+s, p.y-s], [p.x-s, p.y+s], [p.x-s, p.y-s],
  ];
  let best = [p.x, p.y], bestC = Infinity;
  for (const [cx, cy] of cands) {
    const x = clamp(cx, PR, FW-PR), y = clamp(cy, PR, FH-PR);
    const c = v6CostAt(g, p, x, y, sp);
    if (c < bestC) { bestC = c; best = [x, y]; }
  }
  return best;
}

function v6Tick(g, p) {
  const v6 = p.brain.params || {};
  const sp = v6.spatial || {};
  const dec = v6.decisions || BASELINE_AI_PARAMS;
  const ball = g.ball;
  const hasball = ball.owner === p.id;

  // Set-piece taker override
  if (g.setPieceTakerId === p.id && !hasball) {
    const slow = p.slowTimer > 0 ? SLOW_FACTOR : 1;
    moveTo(p, ball.x, ball.y, CSPEED * 1.18 * slow);
    return;
  }

  // Tackle
  if (!hasball && ball.owner !== null) {
    const carrier = g.pl.find(q => q.id === ball.owner);
    if (carrier && carrier.team !== p.team && p.tackleCooldown <= 0) {
      const tc = (dec.tackleChance || 0.08) * (dec.aggression || 1);
      if (Math.hypot(p.x-carrier.x, p.y-carrier.y) < TACKLE_DIST && Math.random() < tc) {
        tacklePlayer(g, p, carrier);
        return;
      }
    }
  }

  // GK + has-ball use classic logic
  if (p.role === 'gk' || hasball) {
    const saved = p.aiPolicy;
    p.aiPolicy = dec;
    cpuTick(g, p);
    p.aiPolicy = saved;
    return;
  }

  // Loose ball chase: closest non-GK pursues
  if (ball.owner === null) {
    let chaserId = null, bestD = Infinity;
    for (const q of g.pl) {
      if (q.role === 'gk' || q.state !== 'active') continue;
      const d = Math.hypot(q.x-ball.x, q.y-ball.y);
      if (d < bestD) { bestD = d; chaserId = q.id; }
    }
    if (chaserId === p.id) {
      const lead = Math.min(18, Math.hypot(ball.vx, ball.vy) * 1.4);
      const tx = clamp(ball.x + ball.vx*lead, PR, FW-PR);
      const ty = clamp(ball.y + ball.vy*lead, PR, FH-PR);
      const slow = p.slowTimer > 0 ? SLOW_FACTOR : 1;
      moveTo(p, tx, ty, CSPEED * 1.18 * slow);
      return;
    }
  }

  // Off-ball: spatial cost minimization
  const [tx, ty] = v6Target(g, p, sp);
  const slow = p.slowTimer > 0 ? SLOW_FACTOR : 1;
  moveTo(p, tx, ty, CSPEED * slow);
}

// v4 in the browser approximates the engine: applies v3's modulation, plus
// the average of the 3 directional multipliers as a coarse pass-chance
// scaling. gkFreedom > 0.5 lets the GK use cpuTick's outfield logic
// (skips the special goal-line handling).
function v4Tick(g, p) {
  const v4 = p.brain.params || {};
  const v3p = v4.v3 || {};
  const avgDirMult = ((v4.passDirOffensive ?? 1) + (v4.passDirDefensive ?? 1) + (v4.passDirNeutral ?? 1)) / 3;

  // Reuse v3 brain logic, then enforce roaming cap.
  const savedBrain = p.brain;
  const savedPolicy = p.aiPolicy;
  p.brain = { version: 'v3', params: v3p };
  v3Tick(g, p);
  p.brain = savedBrain;
  p.aiPolicy = savedPolicy;
  applyRoamLimitJsx(p, v4.maxDistanceFromGoal);
  void avgDirMult;
}

function applyRoamLimitJsx(p, maxDist) {
  if (maxDist == null || maxDist >= 1.0) return;
  const md = Math.max(0, Math.min(1, maxDist));
  const span = FW - 2*FIELD_LINE;
  const limit = md * span;
  if (p.team === 0) {
    if (p.x > FIELD_LINE + limit) p.x = FIELD_LINE + limit;
  } else {
    const minX = FW - FIELD_LINE - limit;
    if (p.x < minX) p.x = minX;
  }
}

function v3Tick(g, p) {
  const b = p.brain.params || {};
  const base = b.base || b;
  const aggression = b.aggression == null ? 1.0 : b.aggression;
  const risk = b.riskAppetite == null ? 0.5 : b.riskAppetite;
  const modulated = Object.assign({}, base, {
    tackleChance: clamp((base.tackleChance ?? 0.08) * aggression, 0.01, 0.5),
    shootProgressThreshold: clamp((base.shootProgressThreshold ?? 0.76) - 0.05 * (risk - 0.5), 0.5, 0.95),
  });
  const saved = p.aiPolicy;
  p.aiPolicy = modulated;
  cpuTick(g, p);
  p.aiPolicy = saved;
}

function cpuTick(g, p) {
  const params = effectivePolicy(g, p);
  const ball = g.ball;
  const hasball = ball.owner===p.id;
  const carrier = g.pl.find(q => q.id===ball.owner);
  const teamHasBall = carrier && carrier.team===p.team;

  // Set-piece taker override: run to the ball
  if (g.setPieceTakerId === p.id && !hasball) {
    const slowMult = p.slowTimer > 0 ? SLOW_FACTOR : 1;
    moveTo(p, ball.x, ball.y, CSPEED * 1.18 * slowMult);
    return;
  }

  if (!hasball && carrier && carrier.team!==p.team && p.tackleCooldown<=0) {
    if (Math.hypot(p.x-carrier.x,p.y-carrier.y)<TACKLE_DIST && Math.random()<(params.tackleChance ?? 0.08)) {
      tacklePlayer(g, p, carrier);
      return;
    }
  }

  if (p.role==='gk') {
    if (p.gkDiveTimer < 0) return; // on ground after missed dive
    if (hasball) {
      if (p.gkHoldTimer > 0) { p.gkHoldTimer--; return; }
      g.gkHasBall[p.team] = false;
      doShoot(g, p, false, FW/2, h2);
      return;
    }
    // Try to dive for incoming shot
    if (p.gkDiveTimer === 0 && ball.owner === null) {
      const isIncoming = p.team===0 ? ball.vx < -8 : ball.vx > 8;
      const goalX = p.team===0 ? FIELD_LINE : FW - FIELD_LINE;
      const distToGoal = Math.abs(p.x - goalX);
      if (isIncoming && distToGoal < GK_DIVE_COMMIT_DIST) {
        const framesUntilGoal = ball.vx !== 0 ? (goalX - ball.x) / ball.vx : 0;
        const predictedY = ball.y + ball.vy * Math.max(0, framesUntilGoal);
        const jitter = GK_DIVE_JITTER * (1 - distToGoal / GK_DIVE_COMMIT_DIST);
        const effectiveY = predictedY + (Math.random()*2-1) * jitter;
        p.gkDiveDir = effectiveY < h2 ? 'up' : 'down';
        p.gkDiveTimer = GK_DIVE_DUR;
      }
    }
    if (p.gkDiveTimer > 0) {
      const diveY = p.gkDiveDir==='up' ? h2 - GH/2 + PR : h2 + GH/2 - PR;
      moveTo(p, p.x, diveY, CSPEED * 3.5);
      p.gkDiveTimer--;
      if (p.gkDiveTimer <= 0) {
        const caught = ball.owner === null &&
          Math.hypot(p.x - ball.x, p.y - ball.y) < PR + BR + 8;
        if (!caught) p.gkDiveTimer = -GK_DIVE_DUR;
        else p.gkDiveTimer = 0;
      }
      return;
    }
    const gx = p.team===0 ? FIELD_LINE + PR*1.5 : FW - FIELD_LINE - PR*1.5;
    moveTo(p, gx, clamp(ball.y, h2-GH/2+PR, h2+GH/2-PR), CSPEED*0.88);
    return;
  }

  // Retreat to midline when opposing GK holds ball
  const enemyTeam = 1 - p.team;
  if (g.gkHasBall[enemyTeam]) {
    const retreatX = p.team===0
      ? Math.min(p.x, FW/2 - PR)
      : Math.max(p.x, FW/2 + PR);
    moveTo(p, retreatX, p.y, CSPEED * 0.9);
    return;
  }

  if (ball.owner===null) {
    const chaser = looseBallChaser(g);
    if (chaser && chaser.id===p.id) {
      const lead = Math.min(18, Math.hypot(ball.vx,ball.vy)*1.4);
      const tx = clamp(ball.x + ball.vx*lead, PR, FW-PR);
      const ty = clamp(ball.y + ball.vy*lead, PR, FH-PR);
      moveTo(p, tx, ty, CSPEED*1.18);
    } else {
      const support = naturalTarget(p, getLooseBallSupportTarget(g,p), p.role==='def'?7:15);
      moveTo(p, support.x, support.y, CSPEED*0.78);
    }
    return;
  }

  if (hasball) {
    // Indirect free kick: must pass first
    if (g.freeKickActive && p.id === g.freeKickShooterId) {
      const pt = cpuFindPass(g, p);
      if (pt) doShoot(g, p, false, pt.x, pt.y, CPU_PASS_POW);
      return;
    }
    const oppGoal = oppGoalPoint(p.team);
    const inShootZone = attackProgress(p.team,p.x)>(params.shootProgressThreshold ?? 0.76);
    const reachedHalf = p.team===0 ? p.x>FW*0.50 : p.x<FW*0.50;
    const onWing = p.role==='mid' && Math.abs(p.y-wingY(p))<54;
    const pressured = nearestOpponentDistance(g,p) < 72;
    const passChance = pressured
      ? (params.passChancePressured ?? 0.16)
      : onWing
        ? (params.passChanceWing ?? 0.07)
        : p.role==='fwd'
          ? (params.passChanceForward ?? 0.04)
          : (params.passChanceDefault ?? 0.055);
    const pt = cpuFindPass(g,p);
    const forwardPt = pt && passMovesForward(p,pt,params.forwardPassMinGain ?? 8) ? pt : null;
    const safePt = pressured ? pt : forwardPt;

    if (p.role==='mid' && !reachedHalf) {
      const lane = naturalTarget(p, { x:clamp(p.x+teamDir(p.team)*100,PR,FW-PR), y:wingY(p) }, 10);
      if (forwardPt && Math.random()<0.04) {
        doShoot(g,p,false,forwardPt.x,forwardPt.y,CPU_PASS_POW);
      }
      moveTo(p, lane.x, lane.y, CSPEED*0.92);
    } else if (safePt && Math.random()<passChance && (!inShootZone || p.role!=='fwd' || Math.random()<0.45)) {
      doShoot(g,p,false,safePt.x,safePt.y,CPU_PASS_POW);
    } else if (inShootZone && (p.role==='fwd' || Math.random()<0.42)) {
      doShoot(g,p,false,oppGoal.x, h2+(p.y-h2)*0.22);
    } else if (safePt && Math.random()<(pressured ? 0.09 : onWing ? 0.04 : 0.025)) {
      if (safePt) doShoot(g,p,false,safePt.x,safePt.y,CPU_PASS_POW);
      else moveTo(p, oppGoal.x, h2+(ball.y-h2)*0.25, CSPEED);
    } else {
      const carryY = p.role==='mid' ? wingY(p) : h2+(ball.y-h2)*0.22;
      const carry = naturalTarget(p, { x:clamp(p.x+teamDir(p.team)*85,PR,FW-PR), y:carryY }, p.role==='mid'?10:18);
      moveTo(p, carry.x, carry.y, CSPEED);
    }
    return;
  }

  const target = teamHasBall ? getAttackTarget(g,p) : getDefendTarget(g,p);
  const loose = p.role==='def' || p.role==='gk' ? 7 : 18;
  const nt = naturalTarget(p, target, loose);
  const slowMult = p.slowTimer > 0 ? SLOW_FACTOR : 1;
  const spd = (teamHasBall ? CSPEED*0.82 : CSPEED) * slowMult;
  moveTo(p, nt.x, nt.y, spd);
}

function resetKickoff(g) {
  const init = newGame();
  g.pl.forEach((p,i) => {
    const s=init.pl[i]; p.x=s.x; p.y=s.y; p.vx=0; p.vy=0;
    p.state='active'; p.knockTimer=0; p.celebrateTimer=0; p.tackleCooldown=0;
    p.jumpTimer=0; p.aiJitterX=0; p.aiJitterY=0; p.aiJitterTimer=0;
    p.facing='down'; p.stepCounter=0;
    p.slowTimer=0; p.gkDiveDir=null; p.gkDiveTimer=0; p.gkHoldTimer=0;
  });
  const b=g.ball;
  b.x=FW/2; b.y=h2; b.vx=0; b.vy=0; b.owner=null; b.mega=false; b.cooldown=0;
  // Conceding team gets the ball at kickoff. goalTeam is set in updateBall
  // when a goal is scored; concedingTeam = the OTHER team.
  if (g.goalTeam === 0 || g.goalTeam === 1) {
    const concedingTeam = g.goalTeam === 0 ? 1 : 0;
    // Hand ball to that team's forward at center spot
    const fwd = g.pl.find(p => p.team === concedingTeam && p.role === 'fwd' && p.state === 'active');
    if (fwd) {
      fwd.x = FW / 2; fwd.y = h2;
      b.x = fwd.x; b.y = fwd.y; b.owner = fwd.id; b.lastTouchTeam = concedingTeam;
      b.cooldown = 16;
    } else {
      b.lastTouchTeam = null;
    }
  } else {
    b.lastTouchTeam = null;
  }
  g.phase='kickoff'; g.celebration=false; g.celebrateFrame=0;
  g.setPieceText=null; g.setPieceTimer=0; g.penaltyTeam=null; g.penaltyTaken=false;
  g.freeKickActive=false; g.freeKickShooterId=null;
  g.gkHasBall=[false,false]; g.setPieceTakerId=null;
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
    gRef.current = newGame();
    gRef.current.teamColors = { ...teamColorsRef.current };

    // Apply per-team selections — fetch in parallel, set brain only for that team
    const opp0 = opponents[selectedIdx];
    const opp1 = opponents[selectedIdx1];
    const fetchOpp = (opp) => opp
      ? fetch(`${opp.file}?t=${Date.now()}`, { cache: 'no-store' })
          .then(r => r.ok ? r.json() : null)
          .catch(() => null)
      : Promise.resolve(null);
    Promise.all([fetchOpp(opp0), fetchOpp(opp1)]).then(([p0, p1]) => {
      if (!gRef.current) return;
      const g = gRef.current;
      let label0 = null, label1 = null;
      if (p0 && opp0) label0 = applyPolicyToTeam(g, opp0, p0, 0);
      if (p1 && opp1) label1 = applyPolicyToTeam(g, opp1, p1, 1);
      g.aiPolicyNames[0] = label0 || g.aiPolicyNames[0];
      g.aiPolicyNames[1] = label1 || g.aiPolicyNames[1];
      const left = (opp0?.label || opp0?.name || 'BASELINE').toUpperCase();
      const right = (opp1?.label || opp1?.name || 'BASELINE').toUpperCase();
      g.setPieceText = `LAG 1: ${left}  vs  LAG 2: ${right}`;
      g.setPieceTimer = 180;
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

    function humanPass() {
      const g=gRef.current;
      if (g.phase!=='playing') return;
      const human=g.pl[0];
      if (human.state!=='active') return;

      if (g.ball.owner===0) {
        const dir = heldPassDirection();
        if (dir) {
          doShoot(g,human,false,human.x+dir.x*180,human.y+dir.y*180,PASS_POW);
          return;
        }
        // Passa till bästa fria lagkamrat, ofta ut på kant om du står centralt
        let best=cpuFindPass(g,human);
        if (!best) {
          let bestD=Infinity;
          g.pl.forEach(p => {
            if (p.id===0||p.team!==0||p.state!=='active') return;
            const d=Math.hypot(p.x-human.x,p.y-human.y);
            if (d<bestD) { bestD=d; best=p; }
          });
        }
        if (best) doShoot(g,human,false,best.x,best.y,PASS_POW);
      }
    }

    function humanTackle() {
      const g=gRef.current;
      if (g.phase!=='playing') return;
      const human=g.pl[0];
      if (human.state!=='active'||human.tackleCooldown>0) return;
      const carrier=g.pl.find(p=>p.id===g.ball.owner);
      if (carrier && carrier.team!==0 && Math.hypot(human.x-carrier.x,human.y-carrier.y)<TACKLE_DIST) {
        tacklePlayer(g, human, carrier);
      } else {
        let best=null,bestD=TACKLE_DIST;
        g.pl.forEach(p => {
          if (p.team===0||p.state!=='active') return;
          const d=Math.hypot(human.x-p.x,human.y-p.y);
          if (d<bestD) { bestD=d; best=p; }
        });
        if (best) {
          tacklePlayer(g, human, best);
        }
      }
    }

    function humanShoot(mega) {
      const g=gRef.current;
      if ((g.phase!=='playing' && g.phase!=='penalty') || g.ball.owner!==0) return;
      // Block direct shot during indirect free kick
      if (g.freeKickActive && g.freeKickShooterId === 0) return;
      const human = g.pl[0];
      if (g.phase==='penalty') {
        const dir = heldPassDirection();
        const tx = dir ? human.x+dir.x*220 : FW-FIELD_LINE;
        const ty = dir ? human.y+dir.y*220 : h2;
        doShoot(g,human,false,tx,ty,SHOOT_POW);
        g.phase='playing';
        g.penaltyTaken=true;
        g.setPieceText=null;
        return;
      }
      doShoot(g,human,mega,FW-FIELD_LINE,h2);
    }

    function humanCelebrate() {
      const g=gRef.current;
      if (g.phase==='goal' && g.celebration) {
        g.pl[0].celebrateTimer = 55;
      }
    }

    function humanJump() {
      const g=gRef.current;
      if (g.phase!=='playing') return;
      const human=g.pl[0];
      if (human.state==='active' && human.jumpTimer<=0) human.jumpTimer=JUMP_DUR;
    }

    const onKD = (e) => {
      const k=e.key.toLowerCase();
      keysRef.current[k]=true;
      const actionKey = k===' ' || k==='w' || k==='e' || k==='enter' ||
        (keysRef.current['w'] && (k==='arrowleft'||k==='arrowright'||k==='arrowup'||k==='arrowdown'));
      if (e.repeat && actionKey) { e.preventDefault(); return; }
      if (k===' ')     { humanShoot(!!keysRef.current['q']); humanCelebrate(); e.preventDefault(); }
      if (k==='w')     { humanPass(); e.preventDefault(); }
      if (keysRef.current['w'] && (k==='arrowleft'||k==='arrowright'||k==='arrowup'||k==='arrowdown')) { humanPass(); e.preventDefault(); }
      if (k==='e')     { humanTackle(); e.preventDefault(); }
      if (k==='enter') { humanJump(); e.preventDefault(); }
      if (k==='backspace') {
        // Toggle human control of player 0. When off, AI plays for them.
        const g = gRef.current;
        if (g && g.pl[0]) {
          g.pl[0].human = !g.pl[0].human;
          g.setPieceText = g.pl[0].human ? 'DU SPELAR' : 'AI TAR ÖVER';
          g.setPieceTimer = 100;
        }
        e.preventDefault();
      }
      if (k==='escape'){ onExit(); }
    };
    const onKU = (e) => { keysRef.current[e.key.toLowerCase()]=false; };
    window.addEventListener('keydown',onKD);
    window.addEventListener('keyup',onKU);

    function update() {
      const g=gRef.current;
      if (g.phase==='kickoff') {
        const anyHuman = g.pl.some(p => p.human);
        if (!anyHuman || Object.values(keysRef.current).some(Boolean)) g.phase='playing';
        return;
      }
      if (g.phase==='goal') {
        g.goalAnim--;
        if (g.celebration) { g.celebrateFrame++; g.pl[0].celebrateTimer = Math.max(0,g.pl[0].celebrateTimer-1); }
        if (g.goalAnim<=0) resetKickoff(g);
        return;
      }
      if (g.phase==='penalty') {
        const shooter = g.pl.find(p => p.id===g.ball.owner);
        if (!shooter) { g.phase='playing'; return; }
        g.ball.x=shooter.x; g.ball.y=shooter.y;
        if (g.penaltyTeam!==0 && !g.penaltyTaken) {
          g.setPieceTimer--;
          if (g.setPieceTimer<=35) {
            const dirY = (Math.random()*2-1)*48;
            const tx = g.penaltyTeam===0 ? FW-FIELD_LINE : FIELD_LINE;
            doShoot(g,shooter,false,tx,h2+dirY,SHOOT_POW);
            g.phase='playing';
            g.penaltyTaken=true;
            g.setPieceText=null;
          }
        }
        return;
      }
      if (g.phase==='fulltime') return;

      g.timer--;
      if (g.timer<=0) { g.phase='fulltime'; return; }
      if (g.setPieceTimer>0) {
        g.setPieceTimer--;
        if (g.setPieceTimer<=0) g.setPieceText=null;
      }

      // Mänsklig rörelse
      const human=g.pl[0];
      if (human.tackleCooldown>0) human.tackleCooldown--;
      if (human.jumpTimer>0) human.jumpTimer--;
      if (human.slowTimer>0) human.slowTimer--;
      if (human.state==='active') {
        const k=keysRef.current;
        const slowMult = human.slowTimer > 0 ? SLOW_FACTOR : 1;
        let dx=0,dy=0;
        if (k['arrowleft'] ||k['a']) dx-=PSPEED*slowMult;
        if (k['arrowright']||k['d']) dx+=PSPEED*slowMult;
        if (k['arrowup']) dy-=PSPEED*slowMult;
        if (k['arrowdown'] ||k['s']) dy+=PSPEED*slowMult;
        if (dx&&dy){dx*=0.707;dy*=0.707;}
        const nx=human.x+dx, ny=human.y+dy;
        human.x=clamp(nx,PR,FW-PR); human.y=clamp(ny,PR,FH-PR);
        if (dx||dy) {
          human.facing=Math.abs(dx)>Math.abs(dy)?(dx>0?'right':'left'):(dy>0?'down':'up');
          human.stepCounter++;
        }
      } else if (--human.knockTimer<=0) { human.state='active'; human.gkDiveDir=null; }

      // CPU
      g.pl.forEach(p => {
        if (p.human) return;
        if (p.tackleCooldown>0) p.tackleCooldown--;
        if (p.jumpTimer>0) p.jumpTimer--;
        if (p.slowTimer>0) p.slowTimer--;
        if (p.gkDiveTimer < 0) { p.gkDiveTimer++; return; } // on ground after missed dive
        if (p.state!=='active') { if (--p.knockTimer<=0) { p.state='active'; p.gkDiveDir=null; } return; }
        tickPlayer(g,p);
      });

      // Boll
      const ball=g.ball;
      if (ball.cooldown>0) ball.cooldown--;

      if (ball.owner!==null) {
        const owner=g.pl.find(p=>p.id===ball.owner);
        if (!owner||owner.state!=='active') { ball.owner=null; }
        else { ball.x=owner.x; ball.y=owner.y; ball.lastTouchTeam=owner.team; }
      } else {
        ball.x+=ball.vx; ball.y+=ball.vy;
        ball.vx*=BALL_FRIC; ball.vy*=BALL_FRIC;

        const inGoalY=Math.abs(ball.y-h2)<GH/2;
        if (ball.x-BR<=FIELD_LINE) {
          if (inGoalY) { g.score[1]++; g.phase='goal'; g.goalAnim=160; g.goalTeam=1; g.lastScorer=null; g.celebration=false; g.gkHasBall[0]=false; try { if (window.SFX) window.SFX.goal(); } catch(e) {} return; }
          handleBallOut(g);
          return;
        }
        if (ball.x+BR>=FW-FIELD_LINE) {
          if (inGoalY) {
            g.score[0]++; g.phase='goal'; g.goalAnim=160; g.goalTeam=0;
            g.lastScorer=0; g.celebration=true; g.celebrateFrame=0; g.gkHasBall[1]=false;
            try { if (window.SFX) window.SFX.goal(); } catch(e) {}
            return;
          }
          handleBallOut(g);
          return;
        }

        if (ball.y-BR<=0 || ball.y+BR>=FH) {
          handleBallOut(g);
          return;
        }

        // Mega knockback
        if (ball.mega) {
          if (ball.vx*ball.vx+ball.vy*ball.vy<20) { ball.mega=false; }
          else {
            g.pl.forEach(p => {
              if (p.state!=='active') return;
              if (Math.hypot(p.x-ball.x,p.y-ball.y)<MEGA_KR) {
                knockPlayer(g, p);
              }
            });
          }
        }

        // Pickup
        if (ball.cooldown<=0) {
          let nearId=null, nearD2=(PR+BR+7)**2;
          g.pl.forEach(p => {
            if (p.state!=='active') return;
            // During an active set piece, only the designated taker can pick up
            if (g.setPieceTakerId !== null && p.id !== g.setPieceTakerId) return;
            const dd=(p.x-ball.x)**2+(p.y-ball.y)**2;
            if (dd<nearD2){nearD2=dd;nearId=p.id;}
          });
          if (nearId!==null) {
            const candidate = g.pl.find(p=>p.id===nearId);
            // GK using hands outside goal area = handball foul → free kick
            if (candidate && candidate.role === 'gk') {
              const inGoalArea = candidate.team===0
                ? candidate.x <= FIELD_LINE + GOAL_AREA_W
                : candidate.x >= FW - FIELD_LINE - GOAL_AREA_W;
              if (!inGoalArea) {
                const oppTeam = 1 - candidate.team;
                const fkTaker = teamPlayers(g, oppTeam)
                  .filter(p => p.role !== 'gk')
                  .sort((a,b)=>Math.hypot(a.x-candidate.x,a.y-candidate.y)-Math.hypot(b.x-candidate.x,b.y-candidate.y))[0]
                  || rolePlayer(g, oppTeam, 'mid');
                if (fkTaker) {
                  awardSetPiece(g, fkTaker.id, candidate.x, candidate.y, 'HANDS!');
                  g.freeKickShooterId = fkTaker.id;
                  g.freeKickActive = true;
                }
                return; // skip normal pickup
              }
            }
            ball.owner=nearId;
            if (g.setPieceTakerId === nearId) g.setPieceTakerId = null;
            const owner=candidate;
            if (owner) {
              ball.lastTouchTeam=owner.team;
              if (g.freeKickActive && owner.id !== g.freeKickShooterId) g.freeKickActive = false;
              if (owner.role === 'gk') {
                g.gkHasBall[owner.team] = true; owner.gkHoldTimer = GK_HOLD_DELAY;
              }
            }
          }
        }
        // Clear gkHasBall if GK no longer has ball
        [0,1].forEach(t => {
          if (g.gkHasBall[t]) {
            const gk = g.pl.find(p => p.role==='gk' && p.team===t);
            if (!gk || ball.owner !== gk.id) g.gkHasBall[t] = false;
          }
        });
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

      // Fulltime — trigger callbacks (React HUD visar statistik-skärmen)
      if (g.phase==='fulltime') {
        if (!g._done) { g._done=true;
          try { if (window.SFX) { window.SFX.whistleFull(); window.SFX.setCrowdTarget(0.06); } } catch(e) {}
          setTimeout(()=>onComplete&&onComplete(g.score[0]>g.score[1],matchData?.id),3500);
        }
      }
    }

    function updateStats() {
      const g = gRef.current;
      if (!g || !g._stats) return;
      if (g.ball.owner !== null) {
        const ownerTeam = g.pl.find(p => p.id === g.ball.owner)?.team;
        if (ownerTeam === 0 || ownerTeam === 1) {
          g._stats._possFrames[ownerTeam]++;
          const total = g._stats._possFrames[0] + g._stats._possFrames[1];
          if (total > 0) {
            g._stats.possession[0] = (g._stats._possFrames[0] / total) * 100;
            g._stats.possession[1] = (g._stats._possFrames[1] / total) * 100;
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
    const loop=()=>{ update(); updateStats(); draw(); raf=requestAnimationFrame(loop); };
    raf=requestAnimationFrame(loop);
    return ()=>{ cancelAnimationFrame(raf); window.removeEventListener('keydown',onKD); window.removeEventListener('keyup',onKU); try { if (window.SFX) window.SFX.stopAll(); } catch(e) {} };
  },[started]);

  const [matchTeam0, setMatchTeam0] = useStateFM(null);
  const [matchTeam1, setMatchTeam1] = useStateFM(null);
  const [gameSnapshot, setGameSnapshot] = useStateFM(null);
  const [matchStats, setMatchStats] = useStateFM(null);
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
      if (g._stats) setMatchStats({ ...g._stats,
        shots: [...g._stats.shots], passes: [...g._stats.passes],
        tackles: [...g._stats.tackles], corners: [...g._stats.corners],
        possession: [...g._stats.possession]
      });
    }, 120);
    return () => clearInterval(id);
  }, [started]);

  const teamColorsRef = useRefFM({ 0: '#3464a8', 1: '#c82828' });

  const handleTeamSelectStart = ({ team0, team1, oppIdx0, oppIdx1 }) => {
    setMatchTeam0(team0);
    setMatchTeam1(team1);
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

      {started && window.MatchHUD && (
        <window.MatchHUD
          game={gameSnapshot}
          team0info={matchTeam0}
          team1info={matchTeam1}
          matchStats={matchStats}
          onExit={onExit}
        />
      )}
    </div>
  );
}

window.FootballMatch = FootballMatch;
