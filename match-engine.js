// Pure match engine for Node/browser simulations. No React, DOM, or canvas.
(function (root, factory) {
  if (typeof module === 'object' && module.exports) module.exports = factory();
  else root.FootballMatchEngine = factory();
})(typeof globalThis !== 'undefined' ? globalThis : this, function () {
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
  const norm = (dx, dy) => {
    const m = Math.hypot(dx, dy) || 1;
    return [dx / m, dy / m];
  };

  function mkP(id, team, x, y, role, human) {
    return {
      id, team, x, y, vx:0, vy:0, role, human:!!human, state:'active',
      knockTimer:0, homeX:x, homeY:y, facing:'down', stepCounter:0,
      celebrateTimer:0, tackleCooldown:0, jumpTimer:0,
      aiJitterX:0, aiJitterY:0, aiJitterTimer:0,
    };
  }

  function createGame(options) {
    const opts = options || {};
    const game = {
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
      aiPolicies:{
        0: {
          passChancePressured: 0.16, passChanceWing: 0.07, passChanceForward: 0.04,
          passChanceDefault: 0.055, shootProgressThreshold: 0.76, tackleChance: 0.08,
          forwardPassMinGain: 8, markDistance: 48,
        },
        1: {
          passChancePressured: 0.16, passChanceWing: 0.07, passChanceForward: 0.04,
          passChanceDefault: 0.055, shootProgressThreshold: 0.76, tackleChance: 0.08,
          forwardPassMinGain: 8, markDistance: 48,
        },
      },
      events: [],
      stats: {
        passes:0, passCompleted:0, shots:0, shotsOnTarget:0,
        goals:0, tackles:0, tackleSuccess:0, turnovers:0, outOfBounds:0,
      },
      _done:false,
    };
    if (opts.aiOnly) game.pl.forEach(p => { p.human = false; });
    return game;
  }

  function emit(g, type, data) {
    const ev = Object.assign({ type, frame: GAME_SECS*60 - g.timer }, data || {});
    g.events.push(ev);
    return ev;
  }

  function doShoot(g, shooter, mega, tx, ty, pow, kind) {
    const p = pow || (mega ? MEGA_POW : SHOOT_POW);
    const ball = g.ball;
    const [nx,ny] = norm(tx - shooter.x, ty - shooter.y);
    ball.vx=nx*p; ball.vy=ny*p;
    ball.x=shooter.x; ball.y=shooter.y;
    ball.owner=null; ball.mega=!!mega; ball.cooldown=BALL_COOL;
    ball.lastTouchTeam=shooter.team;
    if (kind === 'pass') {
      g.stats.passes++;
      emit(g, 'pass_attempt', { team:shooter.team, playerId:shooter.id, targetX:tx, targetY:ty });
    } else {
      g.stats.shots++;
      emit(g, 'shot_attempt', { team:shooter.team, playerId:shooter.id, mega:!!mega });
    }
  }

  function knockPlayer(g, p, duration) {
    if (!p || p.state !== 'active') return;
    p.state = 'knocked';
    p.knockTimer = duration || KNOCK_DUR;
    p.jumpTimer = 0;
    if (g.ball.owner === p.id) {
      g.ball.owner = null;
      g.ball.x = p.x;
      g.ball.y = p.y;
      g.ball.vx = 0;
      g.ball.vy = 0;
      g.ball.mega = false;
      g.ball.cooldown = BALL_COOL;
      g.stats.turnovers++;
      emit(g, 'turnover', { team:p.team, playerId:p.id });
    }
  }

  function isJumping(p) {
    return p && p.state==='active' && p.jumpTimer>0;
  }

  function tacklePlayer(g, tackler, target) {
    if (!tackler || !target || tackler.state!=='active' || target.state!=='active') return false;
    g.stats.tackles++;
    emit(g, 'tackle_attempt', { team:tackler.team, playerId:tackler.id, targetId:target.id });
    if (target.team!==tackler.team && isInOwnPenaltyArea(tackler)) {
      startPenalty(g, target.team);
      tackler.tackleCooldown = TACKLE_COOL;
      emit(g, 'penalty_awarded', { team:target.team, tacklerId:tackler.id });
      return true;
    }
    if (isJumping(target)) {
      knockPlayer(g, tackler, TACKLE_MISS_DUR);
      emit(g, 'tackle_evaded', { team:target.team, playerId:target.id, tacklerId:tackler.id });
    } else {
      knockPlayer(g, target);
      g.stats.tackleSuccess++;
      emit(g, 'tackle_success', { team:tackler.team, playerId:tackler.id, targetId:target.id });
    }
    tackler.tackleCooldown = TACKLE_COOL;
    return true;
  }

  function moveTo(p, tx, ty, speed) {
    const [nx,ny] = norm(tx-p.x, ty-p.y);
    const npx = p.x + nx*speed, npy = p.y + ny*speed;
    p.x=clamp(npx,PR,FW-PR); p.y=clamp(npy,PR,FH-PR);
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
    return x < FW/2 ? { attacking:1, defending:0 } : { attacking:0, defending:1 };
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
    setBallOwner(g, keeper, x, h2, 'MALVAKTENS BOLL');
    emit(g, 'goal_kick', { team });
  }

  function restartKickIn(g, team, x, y) {
    const taker = teamPlayers(g, team)
      .filter(p => p.role!=='gk')
      .sort((a,b)=>Math.hypot(a.x-x,a.y-y)-Math.hypot(b.x-x,b.y-y))[0] || rolePlayer(g, team, 'mid');
    setBallOwner(g, taker, clamp(x, PR, FW-PR), y < h2 ? PR : FH-PR, 'INSPARK');
    emit(g, 'kick_in', { team });
  }

  function restartCorner(g, team, x, y) {
    const taker = rolePlayer(g, team, 'mid') || rolePlayer(g, team, 'fwd');
    setBallOwner(g, taker, x < FW/2 ? PR : FW-PR, y < h2 ? PR : FH-PR, 'HORNA');
    emit(g, 'corner', { team });
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
    g.stats.outOfBounds++;
    emit(g, 'out_of_bounds', { x:b.x, y:b.y, lastTouchTeam:b.lastTouchTeam });
    if (b.y-BR <= 0 || b.y+BR >= FH) {
      const restartTeam = b.lastTouchTeam===0 ? 1 : 0;
      restartKickIn(g, restartTeam, b.x, b.y);
      return true;
    }
    if (b.x-BR <= 0 || b.x+BR >= FW) {
      const teams = goalLineTeams(b.x);
      if (b.lastTouchTeam===teams.attacking) restartGoalKick(g, teams.defending);
      else restartCorner(g, teams.attacking, b.x, b.y);
      return true;
    }
    return false;
  }

  function isMarked(g, p, threshold) {
    return g.pl.some(q => q.team!==p.team && q.state==='active' && Math.hypot(q.x-p.x,q.y-p.y)<(threshold || 55));
  }
  function ownGoalPoint(team) { return { x: team===0 ? 0 : FW, y: h2 }; }
  function oppGoalPoint(team) { return { x: team===0 ? FW+GD : -GD, y: h2 }; }
  function teamDir(team) { return team===0 ? 1 : -1; }
  function attackProgress(team, x) { return team===0 ? x/FW : 1 - x/FW; }
  function sideOf(p) { return p.homeY < h2 ? -1 : 1; }
  function wingY(p) { return sideOf(p) < 0 ? 58 : FH-58; }
  function pointBetween(a, b, t) { return { x:a.x+(b.x-a.x)*t, y:a.y+(b.y-a.y)*t }; }

  function distToSegment(px, py, ax, ay, bx, by) {
    const vx=bx-ax, vy=by-ay;
    const len2=vx*vx+vy*vy || 1;
    const t=clamp(((px-ax)*vx+(py-ay)*vy)/len2,0,1);
    const sx=ax+vx*t, sy=ay+vy*t;
    return Math.hypot(px-sx, py-sy);
  }

  function passLineOpen(g, from, to, team, blockDist) {
    const bd = blockDist || PASS_BLOCK_DIST;
    return !g.pl.some(q => {
      if (q.team===team || q.state!=='active') return false;
      return distToSegment(q.x,q.y,from.x,from.y,to.x,to.y) < bd;
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

  function passMovesForward(from, to, minGain) {
    return (to.x-from.x) * teamDir(from.team) > (minGain || 8);
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

  function shapeXWithBall(p, ballX, strength) {
    return clamp(p.homeX + (ballX - FW/2)*(strength || 0.55), PR, FW-PR);
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

  function naturalTarget(p, target, amp, random) {
    const rng = random || Math.random;
    p.aiJitterTimer--;
    if (p.aiJitterTimer<=0) {
      const a = amp == null ? 16 : amp;
      p.aiJitterX = (rng()*2-1)*a;
      p.aiJitterY = (rng()*2-1)*a;
      p.aiJitterTimer = 35 + Math.floor(rng()*55);
    }
    return { x: clamp(target.x+p.aiJitterX, PR, FW-PR), y: clamp(target.y+p.aiJitterY, PR, FH-PR) };
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
    if (p.role==='fwd') return opponentCarrier ? { x:opponentCarrier.x, y:opponentCarrier.y } : { x:ball.x, y:ball.y };
    if (p.role==='def') return defensiveBlockTarget(g,p,opponentCarrier || {x:ball.x,y:ball.y});
    if (p.role==='mid' && opponentCarrier && Math.hypot(p.x-opponentCarrier.x,p.y-opponentCarrier.y)<190) {
      const block = defensiveBlockTarget(g,p,opponentCarrier);
      const intercept = bestInterceptionTarget(g,p,opponentCarrier);
      return intercept && Math.random()<0.45 ? intercept : block;
    }
    const ratio = p.role==='mid' ? 0.64 : 0.22;
    const bx = ownGoalX + (ball.x-ownGoalX)*ratio;
    const by = h2 + (ball.y-h2)*0.38 + (p.homeY-h2)*0.34;
    return { x:bx, y:clamp(by,PR,FH-PR) };
  }

  function cpuFindPass(g, carrier) {
    const params = g.aiPolicies?.[carrier.team] || {};
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

  function baselineCpuTick(g, p, random) {
    const rng = random || Math.random;
    const params = g.aiPolicies?.[p.team] || {};
    const ball = g.ball;
    const hasball = ball.owner===p.id;
    const carrier = g.pl.find(q => q.id===ball.owner);
    const teamHasBall = carrier && carrier.team===p.team;

    if (!hasball && carrier && carrier.team!==p.team && p.tackleCooldown<=0) {
      if (Math.hypot(p.x-carrier.x,p.y-carrier.y)<TACKLE_DIST && rng()<(params.tackleChance ?? 0.08)) {
        tacklePlayer(g, p, carrier);
        return;
      }
    }

    if (p.role==='gk') {
      if (hasball) { doShoot(g,p,false,FW/2,h2,undefined,'shot'); return; }
      const gx = p.team===0 ? PR*1.8 : FW-PR*1.8;
      moveTo(p, gx, clamp(ball.y,h2-GH/2+PR,h2+GH/2-PR), CSPEED*0.88);
      return;
    }

    if (ball.owner===null) {
      const chaser = looseBallChaser(g);
      if (chaser && chaser.id===p.id) {
        const lead = Math.min(18, Math.hypot(ball.vx,ball.vy)*1.4);
        moveTo(p, clamp(ball.x + ball.vx*lead, PR, FW-PR), clamp(ball.y + ball.vy*lead, PR, FH-PR), CSPEED*1.18);
      } else {
        const support = naturalTarget(p, getLooseBallSupportTarget(g,p), p.role==='def'?7:15, rng);
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
        const lane = naturalTarget(p, { x:clamp(p.x+teamDir(p.team)*100,PR,FW-PR), y:wingY(p) }, 10, rng);
        if (forwardPt && rng()<0.04) doShoot(g,p,false,forwardPt.x,forwardPt.y,CPU_PASS_POW,'pass');
        moveTo(p, lane.x, lane.y, CSPEED*0.92);
      } else if (safePt && rng()<passChance && (!inShootZone || p.role!=='fwd' || rng()<0.45)) {
        doShoot(g,p,false,safePt.x,safePt.y,CPU_PASS_POW,'pass');
      } else if (inShootZone && (p.role==='fwd' || rng()<0.42)) {
        doShoot(g,p,false,oppGoal.x, h2+(p.y-h2)*0.22, undefined, 'shot');
      } else if (safePt && rng()<(pressured ? 0.09 : onWing ? 0.04 : 0.025)) {
        doShoot(g,p,false,safePt.x,safePt.y,CPU_PASS_POW,'pass');
      } else {
        const carryY = p.role==='mid' ? wingY(p) : h2+(ball.y-h2)*0.22;
        const carry = naturalTarget(p, { x:clamp(p.x+teamDir(p.team)*85,PR,FW-PR), y:carryY }, p.role==='mid'?10:18, rng);
        moveTo(p, carry.x, carry.y, CSPEED);
      }
      return;
    }

    const target = teamHasBall ? getAttackTarget(g,p) : getDefendTarget(g,p);
    const loose = p.role==='def' || p.role==='gk' ? 7 : 18;
    const nt = naturalTarget(p, target, loose, rng);
    const spd = teamHasBall ? CSPEED*0.82 : CSPEED;
    moveTo(p, nt.x, nt.y, spd);
  }

  function resetKickoff(g) {
    const init = createGame();
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

  function applyHumanAction(g, action) {
    const human = g.pl[0];
    if (!action || human.state!=='active') return;
    if (action.jump && human.jumpTimer<=0) human.jumpTimer=JUMP_DUR;
    if (action.tackle && human.tackleCooldown<=0) {
      const carrier=g.pl.find(p=>p.id===g.ball.owner);
      if (carrier && carrier.team!==0 && Math.hypot(human.x-carrier.x,human.y-carrier.y)<TACKLE_DIST) tacklePlayer(g, human, carrier);
    }
    if ((action.pass || action.shoot) && g.ball.owner===0) {
      const aim = action.aim || {};
      const dx = aim.x || 1;
      const dy = aim.y || 0;
      if (action.pass) doShoot(g,human,false,human.x+dx*180,human.y+dy*180,PASS_POW,'pass');
      else doShoot(g,human,!!action.mega,human.x+dx*220,human.y+dy*220,undefined,'shot');
    }
    if (action.move && g.phase==='playing') {
      let dx = clamp(action.move.x || 0, -1, 1) * PSPEED;
      let dy = clamp(action.move.y || 0, -1, 1) * PSPEED;
      if (dx&&dy){dx*=0.707;dy*=0.707;}
      human.x=clamp(human.x+dx,PR,FW-PR); human.y=clamp(human.y+dy,PR,FH-PR);
    }
  }

  function updateBall(g) {
    const ball=g.ball;
    if (ball.cooldown>0) ball.cooldown--;
    if (ball.owner!==null) {
      const owner=g.pl.find(p=>p.id===ball.owner);
      if (!owner||owner.state!=='active') ball.owner=null;
      else { ball.x=owner.x; ball.y=owner.y; ball.lastTouchTeam=owner.team; }
      return;
    }

    ball.x+=ball.vx; ball.y+=ball.vy;
    ball.vx*=BALL_FRIC; ball.vy*=BALL_FRIC;

    const inGoalY=Math.abs(ball.y-h2)<GH/2;
    if (ball.x-BR<=0) {
      if (inGoalY) {
        g.score[1]++; g.phase='goal'; g.goalAnim=160; g.goalTeam=1;
        g.stats.goals++; emit(g, 'goal', { team:1 });
        return;
      }
      handleBallOut(g); return;
    }
    if (ball.x+BR>=FW) {
      if (inGoalY) {
        g.score[0]++; g.phase='goal'; g.goalAnim=160; g.goalTeam=0;
        g.stats.goals++; emit(g, 'goal', { team:0 });
        return;
      }
      handleBallOut(g); return;
    }
    if (ball.y-BR<=0 || ball.y+BR>=FH) { handleBallOut(g); return; }

    if (ball.mega) {
      if (ball.vx*ball.vx+ball.vy*ball.vy<20) ball.mega=false;
      else {
        g.pl.forEach(p => {
          if (p.state==='active' && Math.hypot(p.x-ball.x,p.y-ball.y)<MEGA_KR) knockPlayer(g, p);
        });
      }
    }

    if (ball.cooldown<=0) {
      let nearId=null, nearD2=(PR+BR+7)**2;
      g.pl.forEach(p => {
        if (p.state!=='active') return;
        const dd=(p.x-ball.x)**2+(p.y-ball.y)**2;
        if (dd<nearD2){nearD2=dd;nearId=p.id;}
      });
      if (nearId!==null) {
        const prev = ball.lastTouchTeam;
        ball.owner=nearId;
        const owner=g.pl.find(p=>p.id===nearId);
        if (owner) {
          if (prev===owner.team) {
            g.stats.passCompleted++;
            emit(g, 'pass_completed', { team:owner.team, playerId:owner.id });
          }
          ball.lastTouchTeam=owner.team;
        }
      }
    }
  }

  function stepGame(g, input, options) {
    const opts = options || {};
    const rng = opts.random || Math.random;
    const humanAction = input && input.human;
    if (g.phase==='kickoff') g.phase='playing';
    if (g.phase==='goal') {
      g.goalAnim--;
      if (g.goalAnim<=0) resetKickoff(g);
      return g;
    }
    if (g.phase==='penalty') {
      const shooter = g.pl.find(p => p.id===g.ball.owner);
      if (!shooter) { g.phase='playing'; return g; }
      g.ball.x=shooter.x; g.ball.y=shooter.y;
      if (g.penaltyTeam===0 && humanAction && humanAction.shoot) {
        const aim = humanAction.aim || { x:1, y:0 };
        doShoot(g,shooter,false,shooter.x+(aim.x||1)*220,shooter.y+(aim.y||0)*220,SHOOT_POW,'shot');
        g.phase='playing'; g.penaltyTaken=true; g.setPieceText=null;
      } else if (g.penaltyTeam!==0 && !g.penaltyTaken) {
        g.setPieceTimer--;
        if (g.setPieceTimer<=35) {
          const tx = g.penaltyTeam===0 ? FW+GD : -GD;
          doShoot(g,shooter,false,tx,h2+(rng()*2-1)*48,SHOOT_POW,'shot');
          g.phase='playing'; g.penaltyTaken=true; g.setPieceText=null;
        }
      }
      return g;
    }
    if (g.phase==='fulltime') return g;

    g.timer--;
    if (g.timer<=0) { g.phase='fulltime'; return g; }
    if (g.setPieceTimer>0) {
      g.setPieceTimer--;
      if (g.setPieceTimer<=0) g.setPieceText=null;
    }

    const human=g.pl[0];
    if (human.tackleCooldown>0) human.tackleCooldown--;
    if (human.jumpTimer>0) human.jumpTimer--;
    if (human.state!=='active') {
      if (--human.knockTimer<=0) human.state='active';
    } else {
      applyHumanAction(g, humanAction);
    }

    g.pl.forEach(p => {
      if (p.human) return;
      if (p.tackleCooldown>0) p.tackleCooldown--;
      if (p.jumpTimer>0) p.jumpTimer--;
      if (p.state!=='active') { if (--p.knockTimer<=0) p.state='active'; return; }
      const policy = opts.teamPolicies && opts.teamPolicies[p.team] || 'baseline';
      if (policy === 'candidate') candidateCpuTick(g,p,rng,opts.candidatePolicy);
      else baselineCpuTick(g,p,rng);
    });

    updateBall(g);
    return g;
  }

  function runSimulation(options) {
    const opts = options || {};
    const g = createGame({ aiOnly: !!opts.aiOnly });
    const maxFrames = opts.frames || GAME_SECS*60;
    for (let i=0; i<maxFrames && g.phase!=='fulltime'; i++) {
      stepGame(g, null, opts);
    }
    return g;
  }

  function candidateCpuTick(g, p, random, policy) {
    const params = { ...(policy && policy.parameters || {}) };
    if (!g.aiPolicies) g.aiPolicies = {};
    const previous = g.aiPolicies[p.team];
    g.aiPolicies[p.team] = Object.assign({}, previous || {}, params);
    baselineCpuTick(g, p, random);
    g.aiPolicies[p.team] = previous;
  }

  return {
    constants: { FW, FH, GH, GD, PR, BR, GAME_SECS },
    createGame,
    stepGame,
    runSimulation,
    doShoot,
    tacklePlayer,
    cpuFindPass,
    baselineCpuTick,
    candidateCpuTick,
  };
});
