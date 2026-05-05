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

  // Set pieces & new mechanics
  const SLOW_DUR = 6;              // 100ms slowdown after on-ball tackle
  const SLOW_FACTOR = 0.5;         // speed multiplier when slowed
  const FOUL_PAUSE = 30;           // 0.5s freeze after foul before free kick
  const FREE_KICK_WALL_DIST = 55;  // min opponent distance from free kick spot
  const GK_DIVE_DUR = 6;           // 100ms on ground after missed dive
  const GK_DIVE_COMMIT_DIST = 160; // distance from goal at which GK commits dive dir
  const GK_DIVE_JITTER = 40;       // y-prediction noise for close shots
  const GK_HOLD_DELAY = 60;        // 1s GK holds ball before distributing
  const SET_PIECE_DELAY = 60;      // 1s delay before corner/kick-in/goal-kick
  const FIELD_LINE = 18;           // visual field-line offset from canvas edge
  const GOAL_AREA_W = 54;          // goal area width (GK can use hands here)
  const TACKLE_BALL_NUDGE_POW = 6; // ball speed when nudged forward by tackler

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
      slowTimer:0,
      gkDiveDir:null, gkDiveTimer:0, gkHoldTimer:0,
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
      freeKickActive:false, freeKickShooterId:null,
      gkHasBall:[false,false],
      setPieceTakerId:null, setPieceX:0, setPieceY:0,
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
    // Indirect free kick: taker cannot shoot directly, must pass first
    if (g.freeKickActive && shooter.id === g.freeKickShooterId && kind === 'shot') return;
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

  function slowPlayer(p, dur) {
    p.slowTimer = dur || SLOW_DUR;
  }

  function isJumping(p) {
    return p && p.state==='active' && p.jumpTimer>0;
  }

  function tacklePlayer(g, tackler, target) {
    if (!tackler || !target || tackler.state!=='active' || target.state!=='active') return false;
    g.stats.tackles++;
    emit(g, 'tackle_attempt', { team:tackler.team, playerId:tackler.id, targetId:target.id });
    tackler.tackleCooldown = TACKLE_COOL;

    const targetHasBall = g.ball.owner === target.id;

    if (isJumping(target)) {
      knockPlayer(g, tackler, TACKLE_MISS_DUR);
      emit(g, 'tackle_evaded', { team:target.team, playerId:target.id, tacklerId:tackler.id });
      return true;
    }

    if (targetHasBall) {
      // Cannot kick the ball out of a GK's hands — foul, free kick to GK's team
      if (target.role === 'gk' && g.gkHasBall[target.team]) {
        awardSetPiece(g, target.id, target.x, target.y, 'FRISPARK');
        g.gkHasBall[target.team] = false;
        g.stats.fouls = (g.stats.fouls || 0) + 1;
        emit(g, 'foul', { team:tackler.team, playerId:tackler.id, targetId:target.id, x:target.x, y:target.y });
        return true;
      }
      // On-ball tackle: strip the ball, nudge it forward in tackler's direction
      const b = g.ball;
      const [nx, ny] = norm(target.x - tackler.x, target.y - tackler.y);
      b.owner = null;
      b.vx = nx * TACKLE_BALL_NUDGE_POW;
      b.vy = ny * TACKLE_BALL_NUDGE_POW;
      b.x = target.x; b.y = target.y;
      b.cooldown = BALL_COOL;
      slowPlayer(target, SLOW_DUR);
      g.stats.tackleSuccess++;
      emit(g, 'tackle_success_ball', { team:tackler.team, playerId:tackler.id, targetId:target.id });
    } else {
      // Off-ball tackle: foul. No pause, no knock-down — slow the target so
      // they stumble briefly, then the match continues with a free kick.
      if (target.team!==tackler.team && isInOwnPenaltyArea(tackler)) {
        startPenalty(g, target.team);
        emit(g, 'penalty_awarded', { team:target.team, tacklerId:tackler.id });
        return true;
      }
      slowPlayer(target, SLOW_DUR * 4); // brief stumble after foul
      startFreeKick(g, target, target.x, target.y);
      g.stats.tackleSuccess++;
      emit(g, 'foul', { team:tackler.team, playerId:tackler.id, targetId:target.id, x:target.x, y:target.y });
    }
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

  // Returns the active policy params for a player: their per-player override
  // if set, otherwise the team-level policy from g.aiPolicies.
  function effectivePolicy(g, p) {
    return p.aiPolicy || g.aiPolicies?.[p.team] || {};
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
    g.setPieceTakerId=null;
  }

  // Place the ball at a set-piece spot and mark a specific taker. Match continues
  // running normally — only the marked player can pick up the ball, and they
  // run to it via the AI override in baselineCpuTick.
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
    const sx = team===0 ? FIELD_LINE + PR*2.3 : FW - FIELD_LINE - PR*2.3;
    awardSetPiece(g, keeper.id, sx, h2, 'MÅLVAKTENS BOLL');
    g.gkHasBall[team] = false;
    emit(g, 'goal_kick', { team });
  }

  function restartKickIn(g, team, x, y) {
    const taker = teamPlayers(g, team)
      .filter(p => p.role!=='gk')
      .sort((a,b)=>Math.hypot(a.x-x,a.y-y)-Math.hypot(b.x-x,b.y-y))[0] || rolePlayer(g, team, 'mid');
    const sy = y < h2 ? PR : FH-PR;
    awardSetPiece(g, taker.id, clamp(x, PR, FW-PR), sy, 'INSPARK');
    emit(g, 'kick_in', { team });
  }

  function restartCorner(g, team, x, y) {
    const taker = rolePlayer(g, team, 'mid') || rolePlayer(g, team, 'fwd');
    const sx = x < FW/2 ? FIELD_LINE + PR : FW - FIELD_LINE - PR;
    const sy = y < h2 ? PR : FH-PR;
    awardSetPiece(g, taker.id, sx, sy, 'HÖRNA');
    emit(g, 'corner', { team });
  }

  function startFreeKick(g, fouledPlayer, fx, fy) {
    awardSetPiece(g, fouledPlayer.id, fx, fy, 'FRISPARK');
    g.freeKickShooterId = fouledPlayer.id;
    g.freeKickActive = true;
  }

  function startPenalty(g, team) {
    const shooter = team===0 ? g.pl[0] : (rolePlayer(g, team, 'fwd') || rolePlayer(g, team, 'mid'));
    const sx = team===0 ? FW - FIELD_LINE - PENALTY_SPOT_D : FIELD_LINE + PENALTY_SPOT_D;
    // Clear players from penalty area (except shooter and opposing GK)
    g.pl.forEach(p => {
      p.state='active'; p.knockTimer=0; p.jumpTimer=0; p.tackleCooldown=Math.max(p.tackleCooldown, FOUL_PAUSE + SET_PIECE_DELAY);
      if (p.id === shooter.id) return;
      const oppGk = p.role==='gk' && p.team!==team;
      if (!oppGk) {
        // Push field players behind penalty spot (outside penalty area)
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
    g.stats.outOfBounds++;
    emit(g, 'out_of_bounds', { x:b.x, y:b.y, lastTouchTeam:b.lastTouchTeam });
    if (b.y-BR <= 0 || b.y+BR >= FH) {
      const restartTeam = b.lastTouchTeam===0 ? 1 : 0;
      restartKickIn(g, restartTeam, b.x, b.y);
      return true;
    }
    if (b.x-BR <= FIELD_LINE || b.x+BR >= FW-FIELD_LINE) {
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

  function baselineCpuTick(g, p, random) {
    const rng = random || Math.random;
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
      if (Math.hypot(p.x-carrier.x,p.y-carrier.y)<TACKLE_DIST && rng()<(params.tackleChance ?? 0.08)) {
        tacklePlayer(g, p, carrier);
        return;
      }
    }

    if (p.role==='gk') {
      if (p.gkDiveTimer < 0) return; // on the ground after missed dive
      if (hasball) {
        // Hold ball briefly, then distribute
        if (p.gkHoldTimer > 0) {
          p.gkHoldTimer--;
          return;
        }
        g.gkHasBall[p.team] = false;
        doShoot(g, p, false, FW/2, h2, undefined, 'shot');
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
          const effectiveY = predictedY + (rng()*2-1) * jitter;
          p.gkDiveDir = effectiveY < h2 ? 'up' : 'down';
          p.gkDiveTimer = GK_DIVE_DUR;
        }
      }
      if (p.gkDiveTimer > 0) {
        // Actively diving: move quickly toward predicted save position
        const diveY = p.gkDiveDir==='up' ? h2 - GH/2 + PR : h2 + GH/2 - PR;
        moveTo(p, p.x, diveY, CSPEED * 3.5);
        p.gkDiveTimer--;
        if (p.gkDiveTimer <= 0) {
          // Check if we caught the ball
          const caught = ball.owner === null &&
            Math.hypot(p.x - ball.x, p.y - ball.y) < PR + BR + 8;
          if (!caught) p.gkDiveTimer = -GK_DIVE_DUR; // miss: lie on ground
          else p.gkDiveTimer = 0;
        }
        return;
      }
      // Normal GK patrol
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
        moveTo(p, clamp(ball.x + ball.vx*lead, PR, FW-PR), clamp(ball.y + ball.vy*lead, PR, FH-PR), CSPEED*1.18);
      } else {
        const support = naturalTarget(p, getLooseBallSupportTarget(g,p), p.role==='def'?7:15, rng);
        moveTo(p, support.x, support.y, CSPEED*0.78);
      }
      return;
    }

    if (hasball) {
      // Indirect free kick: must pass first, cannot shoot directly
      if (g.freeKickActive && p.id === g.freeKickShooterId) {
        const pt = cpuFindPass(g, p);
        if (pt) doShoot(g, p, false, pt.x, pt.y, CPU_PASS_POW, 'pass');
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
    const slowMult = p.slowTimer > 0 ? SLOW_FACTOR : 1;
    const spd = (teamHasBall ? CSPEED*0.82 : CSPEED) * slowMult;
    moveTo(p, nt.x, nt.y, spd);
  }

  function resetKickoff(g) {
    const init = createGame();
    g.pl.forEach((p,i) => {
      const s=init.pl[i]; p.x=s.x; p.y=s.y; p.vx=0; p.vy=0;
      p.state='active'; p.knockTimer=0; p.celebrateTimer=0; p.tackleCooldown=0;
      p.jumpTimer=0; p.aiJitterX=0; p.aiJitterY=0; p.aiJitterTimer=0;
      p.facing='down'; p.stepCounter=0;
      p.slowTimer=0; p.gkDiveDir=null; p.gkDiveTimer=0; p.gkHoldTimer=0;
    });
    const b=g.ball;
    b.x=FW/2; b.y=h2; b.vx=0; b.vy=0; b.owner=null; b.mega=false; b.cooldown=0; b.lastTouchTeam=null;
    g.phase='kickoff'; g.celebration=false; g.celebrateFrame=0;
    g.setPieceText=null; g.setPieceTimer=0; g.penaltyTeam=null; g.penaltyTaken=false;
    g.freeKickActive=false; g.freeKickShooterId=null;
    g.gkHasBall=[false,false]; g.setPieceTakerId=null;
  }

  function applyHumanAction(g, action) {
    const human = g.pl[0];
    if (!action || human.state!=='active') return;
    if (action.jump && human.jumpTimer<=0) human.jumpTimer=JUMP_DUR;
    if (action.tackle && human.tackleCooldown<=0) {
      // Find nearest opponent within tackle distance (not just ball carrier)
      let bestTarget=null, bestD=TACKLE_DIST;
      g.pl.forEach(p => {
        if (p.team===0 || p.state!=='active') return;
        const d=Math.hypot(human.x-p.x,human.y-p.y);
        if (d<bestD) { bestD=d; bestTarget=p; }
      });
      if (bestTarget) tacklePlayer(g, human, bestTarget);
    }
    if ((action.pass || action.shoot) && g.ball.owner===0) {
      const aim = action.aim || {};
      const dx = aim.x || 1;
      const dy = aim.y || 0;
      if (action.pass) doShoot(g,human,false,human.x+dx*180,human.y+dy*180,PASS_POW,'pass');
      else doShoot(g,human,!!action.mega,human.x+dx*220,human.y+dy*220,undefined,'shot');
    }
    if (action.move && g.phase==='playing') {
      const slowMult = human.slowTimer > 0 ? SLOW_FACTOR : 1;
      let dx = clamp(action.move.x || 0, -1, 1) * PSPEED * slowMult;
      let dy = clamp(action.move.y || 0, -1, 1) * PSPEED * slowMult;
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
    if (ball.x-BR<=FIELD_LINE) {
      if (inGoalY) {
        g.score[1]++; g.phase='goal'; g.goalAnim=160; g.goalTeam=1;
        g.stats.goals++; emit(g, 'goal', { team:1 });
        g.gkHasBall[0]=false;
        return;
      }
      handleBallOut(g); return;
    }
    if (ball.x+BR>=FW-FIELD_LINE) {
      if (inGoalY) {
        g.score[0]++; g.phase='goal'; g.goalAnim=160; g.goalTeam=0;
        g.stats.goals++; emit(g, 'goal', { team:0 });
        g.gkHasBall[1]=false;
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
        // During an active set piece, only the designated taker can pick up
        if (g.setPieceTakerId !== null && p.id !== g.setPieceTakerId) return;
        const dd=(p.x-ball.x)**2+(p.y-ball.y)**2;
        if (dd<nearD2){nearD2=dd;nearId=p.id;}
      });
      if (nearId!==null) {
        const candidate = g.pl.find(p=>p.id===nearId);
        // GK using hands outside goal area = handball foul → free kick to opponents
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
              g.stats.fouls = (g.stats.fouls || 0) + 1;
              emit(g, 'foul_handball', { team:candidate.team, playerId:candidate.id, x:candidate.x, y:candidate.y });
            }
            return; // skip normal pickup
          }
        }
        const prev = ball.lastTouchTeam;
        ball.owner=nearId;
        if (g.setPieceTakerId === nearId) g.setPieceTakerId = null;
        const owner=candidate;
        if (owner) {
          if (prev===owner.team) {
            g.stats.passCompleted++;
            emit(g, 'pass_completed', { team:owner.team, playerId:owner.id });
          }
          ball.lastTouchTeam=owner.team;

          // Cancel indirect free kick restriction once another player touches the ball
          if (g.freeKickActive && owner.id !== g.freeKickShooterId) {
            g.freeKickActive = false;
          }

          // Track GK holding ball with hands (in goal area only — outside is handball foul above)
          if (owner.role === 'gk') {
            g.gkHasBall[owner.team] = true;
            owner.gkHoldTimer = GK_HOLD_DELAY;
          }
        }
      }
    }

    // Clear gkHasBall if ball is no longer owned by GK
    [0,1].forEach(t => {
      if (g.gkHasBall[t]) {
        const gk = g.pl.find(p => p.role==='gk' && p.team===t);
        if (!gk || ball.owner !== gk.id) g.gkHasBall[t] = false;
      }
    });
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
          const tx = g.penaltyTeam===0 ? FW-FIELD_LINE : FIELD_LINE;
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
    if (human.slowTimer>0) human.slowTimer--;
    if (human.state!=='active') {
      if (--human.knockTimer<=0) human.state='active';
    } else {
      applyHumanAction(g, humanAction);
    }

    g.pl.forEach(p => {
      if (p.human) return;
      if (p.tackleCooldown>0) p.tackleCooldown--;
      if (p.jumpTimer>0) p.jumpTimer--;
      if (p.slowTimer>0) p.slowTimer--;
      if (p.gkDiveTimer < 0) { p.gkDiveTimer++; return; } // on ground after missed dive
      if (p.state!=='active') { if (--p.knockTimer<=0) { p.state='active'; p.gkDiveDir=null; } return; }
      const policy = opts.teamPolicies && opts.teamPolicies[p.team] || 'baseline';
      if (policy === 'candidate') candidateCpuTick(g,p,rng,opts.candidatePolicy);
      else tickPlayer(g,p,rng);
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

  // Dispatcher: every CPU player ticks through here. v1/v2 use classic;
  // v3 modulates; v4 = v3 + directional pass mult + GK freedom.
  function tickPlayer(g, p, random) {
    if (!p.brain) { baselineCpuTick(g, p, random); return; }
    switch (p.brain.version) {
      case 'v6': v6Tick(g, p, random); return;
      case 'v4': v4Tick(g, p, random); return;
      case 'v3': v3Tick(g, p, random); return;
      case 'v1':
      case 'v2':
      default: {
        const saved = p.aiPolicy;
        p.aiPolicy = p.brain.params;
        baselineCpuTick(g, p, random);
        p.aiPolicy = saved;
        return;
      }
    }
  }

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
    return v6PrefCost(dGoal, sp.ownGoal)
         + v6PrefCost(y, sp.side)
         + v6PrefCost(Math.hypot(x-g.ball.x, y-g.ball.y), sp.ball)
         + v6PrefCost(v6NearestActive(g, p.id, p.team, x, y), sp.teammate)
         + v6PrefCost(v6NearestActive(g, p.id, 1-p.team, x, y), sp.opponent);
  }
  function v6Target(g, p, sp) {
    const r = 24, s = r*0.7071;
    const cands = [
      [p.x,p.y],[p.x+r,p.y],[p.x-r,p.y],[p.x,p.y+r],[p.x,p.y-r],
      [p.x+s,p.y+s],[p.x+s,p.y-s],[p.x-s,p.y+s],[p.x-s,p.y-s],
    ];
    let best = [p.x,p.y], bestC = Infinity;
    for (const [cx,cy] of cands) {
      const x=clamp(cx,PR,FW-PR), y=clamp(cy,PR,FH-PR);
      const c = v6CostAt(g, p, x, y, sp);
      if (c < bestC) { bestC=c; best=[x,y]; }
    }
    return best;
  }
  function v6Tick(g, p, random) {
    const rng = random || Math.random;
    const v6 = p.brain.params || {};
    const sp = v6.spatial || {};
    const dec = v6.decisions || {};
    const ball = g.ball;
    const hasball = ball.owner === p.id;

    if (g.setPieceTakerId === p.id && !hasball) {
      const slow = p.slowTimer > 0 ? SLOW_FACTOR : 1;
      moveTo(p, ball.x, ball.y, CSPEED * 1.18 * slow);
      return;
    }
    if (!hasball && ball.owner !== null) {
      const carrier = g.pl.find(q => q.id === ball.owner);
      if (carrier && carrier.team !== p.team && p.tackleCooldown <= 0) {
        const tc = (dec.tackleChance || 0.08) * (dec.aggression || 1);
        if (Math.hypot(p.x-carrier.x,p.y-carrier.y) < TACKLE_DIST && rng() < tc) {
          tacklePlayer(g, p, carrier); return;
        }
      }
    }
    if (p.role === 'gk' || hasball) {
      const saved = p.aiPolicy;
      p.aiPolicy = dec;
      baselineCpuTick(g, p, rng);
      p.aiPolicy = saved;
      return;
    }
    if (ball.owner === null) {
      let chaserId = null, bestD = Infinity;
      for (const q of g.pl) {
        if (q.role === 'gk' || q.state !== 'active') continue;
        const d = Math.hypot(q.x-ball.x, q.y-ball.y);
        if (d < bestD) { bestD = d; chaserId = q.id; }
      }
      if (chaserId === p.id) {
        const lead = Math.min(18, Math.hypot(ball.vx,ball.vy) * 1.4);
        const tx = clamp(ball.x + ball.vx*lead, PR, FW-PR);
        const ty = clamp(ball.y + ball.vy*lead, PR, FH-PR);
        const slow = p.slowTimer > 0 ? SLOW_FACTOR : 1;
        moveTo(p, tx, ty, CSPEED * 1.18 * slow);
        return;
      }
    }
    const [tx, ty] = v6Target(g, p, sp);
    const slow = p.slowTimer > 0 ? SLOW_FACTOR : 1;
    moveTo(p, tx, ty, CSPEED * slow);
  }

  // v4 approximation in JS engine: delegates to v3 logic, then enforces the
  // per-slot maxDistanceFromGoal roaming cap.
  function v4Tick(g, p, random) {
    const v4 = p.brain.params || {};
    const savedBrain = p.brain;
    const savedPolicy = p.aiPolicy;
    p.brain = { version: 'v3', params: v4.v3 || {} };
    v3Tick(g, p, random);
    p.brain = savedBrain;
    p.aiPolicy = savedPolicy;
    applyRoamLimit(p, v4.maxDistanceFromGoal);
  }

  function applyRoamLimit(p, maxDist) {
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

  // v3 algorithm. Today: classic logic + aggression / risk modulators.
  // Stays a single source-of-truth for the v3 dispatch path; future v3 work
  // can diverge further from classic_tick without touching v1/v2.
  function v3Tick(g, p, random) {
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
    baselineCpuTick(g, p, random);
    p.aiPolicy = saved;
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
    constants: { FW, FH, GH, GD, PR, BR, GAME_SECS, FIELD_LINE, GOAL_AREA_W,
                 SLOW_DUR, SLOW_FACTOR, FREE_KICK_WALL_DIST, SET_PIECE_DELAY,
                 GK_DIVE_COMMIT_DIST, GK_DIVE_JITTER, GK_HOLD_DELAY },
    createGame,
    stepGame,
    runSimulation,
    doShoot,
    tacklePlayer,
    cpuFindPass,
    baselineCpuTick,
    candidateCpuTick,
    tickPlayer,
    v3Tick,
    v4Tick,
  };
});
