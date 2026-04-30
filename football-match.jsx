const { useEffect: useEffFM, useRef: useRefFM, useState: useStateFM } = React;

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

function drawPlayer(ctx, p, game) {
  const hasBall  = game.ball.owner === p.id;
  const teamColor = p.team === 0 ? '#3464a8' : '#c82828';
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
  ctx.restore();
}

// ── Spelstatus ────────────────────────────────────────────────────────────────

function mkP(id, team, x, y, role, human=false) {
  return { id, team, x, y, vx:0, vy:0, role, human, state:'active',
           knockTimer:0, homeX:x, homeY:y, facing:'down',
           stepCounter:0, hairColor: HAIR[id], celebrateTimer:0, tackleCooldown:0, jumpTimer:0,
           aiJitterX:0, aiJitterY:0, aiJitterTimer:0 };
}

function newGame() {
  return {
    pl: [
      mkP(0,0,FW*.44,h2,    'fwd',true),
      mkP(1,0,FW*.32,h2-85, 'mid'),
      mkP(2,0,FW*.32,h2+85, 'mid'),
      mkP(3,0,FW*.17,h2,    'def'),
      mkP(4,0,FW*.04,h2,    'gk'),
      mkP(5,1,FW*.56,h2,    'fwd'),
      mkP(6,1,FW*.68,h2-85, 'mid'),
      mkP(7,1,FW*.68,h2+85, 'mid'),
      mkP(8,1,FW*.83,h2,    'def'),
      mkP(9,1,FW*.96,h2,    'gk'),
    ],
    ball:{ x:FW/2,y:h2,vx:0,vy:0,owner:null,mega:false,cooldown:0,lastTouchTeam:null },
    score:[0,0], timer:GAME_SECS*60,
    phase:'kickoff', goalAnim:0, goalTeam:null,
    setPieceText:null, setPieceTimer:0, penaltyTeam:null, penaltyTaken:false,
    aiPolicies:{ 0: BASELINE_AI_PARAMS, 1: BASELINE_AI_PARAMS },
    aiPolicyNames:{ 0: 'baseline', 1: 'baseline' },
    lastScorer:null, celebration:false, celebrateFrame:0,
    _done:false,
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
  if (target.team!==tackler.team && isInOwnPenaltyArea(tackler)) {
    startPenalty(g, target.team);
    tackler.tackleCooldown = TACKLE_COOL;
    return true;
  }
  if (isJumping(target)) {
    knockPlayer(g, tackler, TACKLE_MISS_DUR);
  } else {
    knockPlayer(g, target);
  }
  tackler.tackleCooldown = TACKLE_COOL;
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

function restartGoalKick(g, team) {
  const keeper = rolePlayer(g, team, 'gk');
  const x = team===0 ? PR*2.3 : FW-PR*2.3;
  setBallOwner(g, keeper, x, h2, 'MÅLVAKTENS BOLL');
}

function restartKickIn(g, team, x, y) {
  const taker = teamPlayers(g, team)
    .filter(p => p.role!=='gk')
    .sort((a,b)=>Math.hypot(a.x-x,a.y-y)-Math.hypot(b.x-x,b.y-y))[0] || rolePlayer(g, team, 'mid');
  setBallOwner(g, taker, clamp(x, PR, FW-PR), y < h2 ? PR : FH-PR, 'INSPARK');
}

function restartCorner(g, team, x, y) {
  const taker = rolePlayer(g, team, 'mid') || rolePlayer(g, team, 'fwd');
  setBallOwner(g, taker, x < FW/2 ? PR : FW-PR, y < h2 ? PR : FH-PR, 'HÖRNA');
}

function startPenalty(g, team) {
  const shooter = team===0 ? g.pl[0] : (rolePlayer(g, team, 'fwd') || rolePlayer(g, team, 'mid'));
  const x = team===0 ? FW-PENALTY_SPOT_D : PENALTY_SPOT_D;
  g.pl.forEach(p => {
    p.state='active'; p.knockTimer=0; p.jumpTimer=0; p.tackleCooldown=Math.max(p.tackleCooldown, 35);
  });
  setBallOwner(g, shooter, x, h2, 'STRAFF');
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

  if (b.x-BR <= 0 || b.x+BR >= FW) {
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

function cpuTick(g, p) {
  const params = effectivePolicy(g, p);
  const ball = g.ball;
  const hasball = ball.owner===p.id;
  const carrier = g.pl.find(q => q.id===ball.owner);
  const teamHasBall = carrier && carrier.team===p.team;

  if (!hasball && carrier && carrier.team!==p.team && p.tackleCooldown<=0) {
    if (Math.hypot(p.x-carrier.x,p.y-carrier.y)<TACKLE_DIST && Math.random()<(params.tackleChance ?? 0.08)) {
      tacklePlayer(g, p, carrier);
      return;
    }
  }

  if (p.role==='gk') {
    if (hasball) { doShoot(g,p,false,FW/2,h2); return; }
    const gx = p.team===0 ? PR*1.8 : FW-PR*1.8;
    moveTo(p, gx, clamp(ball.y,h2-GH/2+PR,h2+GH/2-PR), CSPEED*0.88);
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
  const spd = teamHasBall ? CSPEED*0.82 : CSPEED;
  moveTo(p, nt.x, nt.y, spd);
}

function resetKickoff(g) {
  const init = newGame();
  g.pl.forEach((p,i) => {
    const s=init.pl[i]; p.x=s.x; p.y=s.y; p.vx=0; p.vy=0;
    p.state='active'; p.knockTimer=0; p.celebrateTimer=0; p.tackleCooldown=0;
    p.jumpTimer=0; p.aiJitterX=0; p.aiJitterY=0; p.aiJitterTimer=0;
    p.facing='down'; p.stepCounter=0;
  });
  const b=g.ball;
  b.x=FW/2; b.y=h2; b.vx=0; b.vy=0; b.owner=null; b.mega=false; b.cooldown=0; b.lastTouchTeam=null;
  g.phase='kickoff'; g.celebration=false; g.celebrateFrame=0;
  g.setPieceText=null; g.setPieceTimer=0; g.penaltyTeam=null; g.penaltyTaken=false;
}

// ── React-komponent ───────────────────────────────────────────────────────────

function FootballMatch({ matchData, onComplete, onExit }) {
  const canvasRef = useRefFM(null);
  const gRef      = useRefFM(null);
  const keysRef   = useRefFM({});

  const [opponents, setOpponents] = useStateFM([]);
  const [selectedIdx, setSelectedIdx] = useStateFM(0);
  const [started, setStarted] = useStateFM(false);

  // Load opponent list on first mount
  useEffFM(() => {
    fetch('data/policies/opponents.json')
      .then(r => r.ok ? r.json() : { opponents: [] })
      .then(d => setOpponents((d && d.opponents) || []))
      .catch(() => setOpponents([]));
  }, []);

  useEffFM(() => {
    if (!started) return;
    gRef.current = newGame();
    const opp = opponents[selectedIdx];
    if (opp) {
      fetch(opp.file)
        .then(r => r.ok ? r.json() : null)
        .then(policy => {
          if (!policy || !gRef.current) return;
          const params = { ...BASELINE_AI_PARAMS, ...(policy.parameters || {}) };
          gRef.current.aiPolicies[0] = params;
          gRef.current.aiPolicies[1] = params;
          gRef.current.aiPolicyNames[1] = policy.name || opp.name || 'candidate';
          gRef.current.setPieceText = `MOTSTÅNDARE: ${(opp.label || policy.name || opp.name || 'CANDIDATE').toUpperCase()}`;
          gRef.current.setPieceTimer = 120;
        })
        .catch(() => {});
    }
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
      const human = g.pl[0];
      if (g.phase==='penalty') {
        const dir = heldPassDirection();
        const tx = dir ? human.x+dir.x*220 : FW+GD;
        const ty = dir ? human.y+dir.y*220 : h2;
        doShoot(g,human,false,tx,ty,SHOOT_POW);
        g.phase='playing';
        g.penaltyTaken=true;
        g.setPieceText=null;
        return;
      }
      doShoot(g,human,mega,FW+GD,h2);
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
      if (k==='escape'){ onExit(); }
    };
    const onKU = (e) => { keysRef.current[e.key.toLowerCase()]=false; };
    window.addEventListener('keydown',onKD);
    window.addEventListener('keyup',onKU);

    function update() {
      const g=gRef.current;
      if (g.phase==='kickoff') {
        if (Object.values(keysRef.current).some(Boolean)) g.phase='playing';
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
            const tx = g.penaltyTeam===0 ? FW+GD : -GD;
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
      if (human.state==='active') {
        const k=keysRef.current;
        let dx=0,dy=0;
        if (k['arrowleft'] ||k['a']) dx-=PSPEED;
        if (k['arrowright']||k['d']) dx+=PSPEED;
        if (k['arrowup']) dy-=PSPEED;
        if (k['arrowdown'] ||k['s']) dy+=PSPEED;
        if (dx&&dy){dx*=0.707;dy*=0.707;}
        const nx=human.x+dx, ny=human.y+dy;
        human.x=clamp(nx,PR,FW-PR); human.y=clamp(ny,PR,FH-PR);
        if (dx||dy) {
          human.facing=Math.abs(dx)>Math.abs(dy)?(dx>0?'right':'left'):(dy>0?'down':'up');
          human.stepCounter++;
        }
      } else if (--human.knockTimer<=0) { human.state='active'; }

      // CPU
      g.pl.forEach(p => {
        if (p.human) return;
        if (p.tackleCooldown>0) p.tackleCooldown--;
        if (p.jumpTimer>0) p.jumpTimer--;
        if (p.state!=='active') { if (--p.knockTimer<=0) p.state='active'; return; }
        cpuTick(g,p);
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
        if (ball.x-BR<=0) {
          if (inGoalY) { g.score[1]++; g.phase='goal'; g.goalAnim=160; g.goalTeam=1; g.lastScorer=null; g.celebration=false; return; }
          handleBallOut(g);
          return;
        }
        if (ball.x+BR>=FW) {
          if (inGoalY) {
            g.score[0]++; g.phase='goal'; g.goalAnim=160; g.goalTeam=0;
            g.lastScorer=0; g.celebration=true; g.celebrateFrame=0;
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
            const dd=(p.x-ball.x)**2+(p.y-ball.y)**2;
            if (dd<nearD2){nearD2=dd;nearId=p.id;}
          });
          if (nearId!==null) {
            ball.owner=nearId;
            const owner=g.pl.find(p=>p.id===nearId);
            if (owner) ball.lastTouchTeam=owner.team;
          }
        }
      }
    }

    function draw() {
      const g=gRef.current;

      // Plan
      ctx.fillStyle='#2a6318'; ctx.fillRect(0,0,FW,FH);
      for (let i=0;i<11;i++) {
        ctx.fillStyle=i%2===0?'rgba(0,0,0,0.06)':'rgba(255,255,255,0.025)';
        ctx.fillRect(i*(FW/11),0,FW/11,FH);
      }
      ctx.strokeStyle='rgba(255,255,255,0.82)'; ctx.lineWidth=2;
      ctx.strokeRect(18,8,FW-36,FH-16);
      ctx.beginPath();ctx.moveTo(FW/2,8);ctx.lineTo(FW/2,FH-8);ctx.stroke();
      ctx.beginPath();ctx.arc(FW/2,h2,62,0,Math.PI*2);ctx.stroke();
      ctx.fillStyle='rgba(255,255,255,0.8)';
      ctx.beginPath();ctx.arc(FW/2,h2,3,0,Math.PI*2);ctx.fill();
      [[18,h2-88,106,176],[FW-124,h2-88,106,176],[18,h2-46,54,92],[FW-72,h2-46,54,92]].forEach(
        ([x,y,w,hh])=>ctx.strokeRect(x,y,w,hh));

      // Mål
      [[-GD,h2-GH/2],[FW,h2-GH/2]].forEach(([gx,gy])=>{
        ctx.fillStyle='rgba(255,255,255,0.1)'; ctx.fillRect(gx,gy,GD,GH);
        ctx.strokeStyle='rgba(255,255,255,0.9)'; ctx.lineWidth=3; ctx.strokeRect(gx,gy,GD,GH);
      });

      // Spelare (sorterade på y för djup-ordning)
      [...g.pl].sort((a,b)=>a.y-b.y).forEach(p => drawPlayer(ctx,p,g));

      // Boll
      const b=g.ball;
      ctx.save(); ctx.translate(b.x,b.y);
      if (b.mega) {
        const grd=ctx.createRadialGradient(0,0,BR,0,0,BR*3.8);
        grd.addColorStop(0,'rgba(255,210,40,0.95)');
        grd.addColorStop(1,'rgba(255,80,0,0)');
        ctx.fillStyle=grd; ctx.beginPath();ctx.arc(0,0,BR*3.8,0,Math.PI*2);ctx.fill();
      }
      ctx.fillStyle='rgba(0,0,0,0.2)'; ctx.beginPath();ctx.ellipse(2,BR+1,BR*0.8,BR*0.28,0,0,Math.PI*2);ctx.fill();
      ctx.fillStyle=b.mega?'#ffd43b':'#fff';
      ctx.beginPath();ctx.arc(0,0,b.mega?BR*1.6:BR,0,Math.PI*2);ctx.fill();
      ctx.strokeStyle='#1a1a1a';ctx.lineWidth=1.5;ctx.stroke();
      if (!b.mega){ctx.fillStyle='rgba(0,0,0,0.2)';ctx.beginPath();ctx.arc(2,-2,BR*0.4,0,Math.PI*2);ctx.fill();}
      ctx.restore();

      // HUD
      ctx.fillStyle='rgba(0,0,0,0.68)'; ctx.fillRect(FW/2-95,7,190,42);
      ctx.strokeStyle='rgba(255,255,255,0.2)';ctx.lineWidth=1;ctx.strokeRect(FW/2-95,7,190,42);
      ctx.textAlign='center';ctx.textBaseline='middle';
      ctx.font='bold 22px ui-monospace,monospace'; ctx.fillStyle='#fff';
      ctx.fillText(`${g.score[0]}  –  ${g.score[1]}`,FW/2,24);
      const mm=Math.floor(g.timer/3600),ss=Math.floor((g.timer%3600)/60);
      ctx.font='11px ui-monospace,monospace'; ctx.fillStyle='rgba(255,255,255,0.65)';
      ctx.fillText(`${mm}:${String(ss).padStart(2,'0')}`,FW/2,40);
      ctx.font='bold 9px ui-monospace,monospace';
      ctx.fillStyle='#6ea8fe';ctx.textAlign='left'; ctx.fillText('DU',FW/2-92,22);
      ctx.fillStyle='#f87171';ctx.textAlign='right';ctx.fillText('MOTST.',FW/2+92,22);

      // Kontrolltips
      ctx.fillStyle='rgba(0,0,0,0.55)'; ctx.fillRect(0,FH-26,FW,26);
      ctx.font='10px ui-monospace,monospace'; ctx.fillStyle='rgba(255,255,255,0.55)';
      ctx.textAlign='center';
      ctx.fillText('PILAR/ASD rörelse  ·  W passa  ·  W+PIL riktad pass  ·  SPACE skjut  ·  Q+SPACE superskott  ·  E tackling  ·  ENTER hopp',FW/2,FH-12);

      if (g.setPieceText) {
        ctx.fillStyle='rgba(0,0,0,0.62)';
        ctx.fillRect(FW/2-120,58,240,34);
        ctx.strokeStyle='rgba(255,255,255,0.22)'; ctx.strokeRect(FW/2-120,58,240,34);
        ctx.font='bold 16px ui-monospace,monospace'; ctx.fillStyle='#ffd43b';
        ctx.textAlign='center'; ctx.textBaseline='middle';
        ctx.fillText(g.setPieceText,FW/2,75);
        if (g.phase==='penalty' && g.penaltyTeam===0) {
          ctx.font='10px ui-monospace,monospace'; ctx.fillStyle='rgba(255,255,255,0.75)';
          ctx.fillText('Håll pil för riktning och tryck SPACE',FW/2,102);
        }
      }

      // Fasöverlägg
      if (g.phase==='kickoff') {
        ctx.fillStyle='rgba(0,0,0,0.58)'; ctx.fillRect(0,0,FW,FH);
        ctx.font='bold 40px Georgia,serif'; ctx.fillStyle='#fff';
        ctx.textAlign='center'; ctx.textBaseline='middle';
        ctx.fillText('AVSPARK',FW/2,h2-22);
        ctx.font='16px ui-monospace,monospace'; ctx.fillStyle='rgba(255,255,255,0.7)';
        ctx.fillText('Tryck valfri tangent',FW/2,h2+18);
        ctx.font='12px ui-monospace,monospace'; ctx.fillStyle='#ffd43b';
        ctx.fillText(`${matchData?.name||'Match'} · Du spelar i BLÅ`,FW/2,h2+50);
      }

      if (g.phase==='goal') {
        const t=g.goalAnim/160;
        ctx.fillStyle=`rgba(0,0,0,${(1-t)*0.5})`; ctx.fillRect(0,0,FW,FH);
        ctx.globalAlpha=Math.min(1,t*5);
        ctx.font='bold 84px Georgia,serif';
        ctx.fillStyle=g.goalTeam===0?'#ffd43b':'#ff4040';
        ctx.textAlign='center'; ctx.textBaseline='middle';
        ctx.fillText('MÅÅÅL!',FW/2,h2);
        ctx.font='18px ui-monospace,monospace'; ctx.fillStyle='#fff';
        ctx.fillText(g.goalTeam===0?'Ditt lag poängsätter!':'Motståndarlaget poängsätter!',FW/2,h2+60);
        if (g.celebration) {
          ctx.font='13px ui-monospace,monospace'; ctx.fillStyle='#ffd43b';
          ctx.fillText('Tryck SPACE för att fira!',FW/2,h2+90);
        }
        ctx.globalAlpha=1;
      }

      if (g.phase==='fulltime') {
        const won=g.score[0]>g.score[1], draw=g.score[0]===g.score[1];
        ctx.fillStyle='rgba(0,0,0,0.78)'; ctx.fillRect(0,0,FW,FH);
        ctx.font='bold 62px Georgia,serif';
        ctx.fillStyle=won?'#ffd43b':draw?'#fff':'#ff5555';
        ctx.textAlign='center'; ctx.textBaseline='middle';
        ctx.fillText(won?'VINST!':draw?'OAVGJORT':'FÖRLUST',FW/2,h2-28);
        ctx.font='bold 32px ui-monospace,monospace'; ctx.fillStyle='#fff';
        ctx.fillText(`${g.score[0]} – ${g.score[1]}`,FW/2,h2+30);
        ctx.font='13px ui-monospace,monospace'; ctx.fillStyle='rgba(255,255,255,0.55)';
        ctx.fillText('Tryck ESC för att lämna…',FW/2,h2+80);
        if (!g._done) { g._done=true; setTimeout(()=>onComplete&&onComplete(won,matchData?.id),3500); }
      }
    }

    let raf;
    const loop=()=>{ update(); draw(); raf=requestAnimationFrame(loop); };
    raf=requestAnimationFrame(loop);
    return ()=>{ cancelAnimationFrame(raf); window.removeEventListener('keydown',onKD); window.removeEventListener('keyup',onKU); };
  },[started]);

  return (
    <div style={{position:'absolute',inset:0,background:'#0a0a0a',
      display:'flex',alignItems:'center',justifyContent:'center',overflow:'hidden'}}>
      {!started && (
        <div style={{position:'absolute',inset:0,background:'rgba(0,0,0,0.85)',
          display:'flex',flexDirection:'column',alignItems:'center',justifyContent:'center',
          gap:'16px',padding:'24px',color:'#f3f4f6',zIndex:10,fontFamily:'Arial, sans-serif'}}>
          <h2 style={{margin:0,fontSize:'28px',letterSpacing:'1px'}}>VÄLJ MOTSTÅNDARE</h2>
          {opponents.length === 0 ? (
            <p style={{color:'#9ca3af',fontSize:'14px'}}>Laddar motståndare...</p>
          ) : (
            <select
              value={selectedIdx}
              onChange={(e)=>setSelectedIdx(Number(e.target.value))}
              style={{padding:'10px 14px',fontSize:'15px',minWidth:'320px',
                background:'#1f2937',color:'#f3f4f6',border:'1px solid #374151',borderRadius:'6px'}}
            >
              {opponents.map((o, i) => (
                <option key={o.name || i} value={i}>{o.label || o.name}</option>
              ))}
            </select>
          )}
          <button
            onClick={()=>setStarted(true)}
            disabled={opponents.length === 0}
            style={{padding:'12px 28px',fontSize:'16px',fontWeight:700,
              background:'#16a34a',color:'#ffffff',border:'none',borderRadius:'6px',
              cursor: opponents.length === 0 ? 'not-allowed' : 'pointer',
              opacity: opponents.length === 0 ? 0.5 : 1}}
          >
            STARTA MATCH
          </button>
          {onExit && (
            <button onClick={onExit}
              style={{padding:'8px 18px',fontSize:'13px',background:'transparent',
                color:'#9ca3af',border:'1px solid #374151',borderRadius:'6px',cursor:'pointer'}}>
              Avbryt
            </button>
          )}
        </div>
      )}
      <canvas ref={canvasRef} width={FW} height={FH}
        style={{maxWidth:'100%',maxHeight:'100%',display:'block'}}/>
    </div>
  );
}

window.FootballMatch = FootballMatch;
