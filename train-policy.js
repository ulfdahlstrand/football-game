const fs = require('fs');
const path = require('path');
const Engine = require('./match-engine');

const EPOCHS = Number(process.argv[2] || 100);
const GAMES_PER_EPOCH = Number(process.argv[3] || 1000);
const SESSION_NAME = process.argv[4] || 'session-1';

const POLICY_DIR = path.join(__dirname, 'data', 'policies');
const SESSION_DIR = path.join(POLICY_DIR, 'sessions', SESSION_NAME);
const BASELINE_PATH = path.join(POLICY_DIR, 'baseline.json');

const PARAM_BOUNDS = {
  passChancePressured: [0.02, 0.4],
  passChanceWing: [0.01, 0.25],
  passChanceForward: [0.005, 0.18],
  passChanceDefault: [0.005, 0.2],
  shootProgressThreshold: [0.55, 0.9],
  tackleChance: [0.01, 0.22],
  forwardPassMinGain: [0, 18],
  markDistance: [25, 85],
};

const MUTATION_SCALE = {
  passChancePressured: 0.035,
  passChanceWing: 0.025,
  passChanceForward: 0.018,
  passChanceDefault: 0.018,
  shootProgressThreshold: 0.035,
  tackleChance: 0.025,
  forwardPassMinGain: 2,
  markDistance: 5,
};

const WINDOW_MIN_GAMES = 100;
const WINDOW_CHECK_EVERY = 25;
const Z_EARLY_REJECT = 2.5;
const Z_EARLY_ACCEPT = 3.0;

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function writeJson(file, data) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${JSON.stringify(data, null, 2)}\n`);
}

function clamp(v, lo, hi) {
  return Math.max(lo, Math.min(hi, v));
}

function randNormal() {
  let u = 0, v = 0;
  while (u === 0) u = Math.random();
  while (v === 0) v = Math.random();
  return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
}

function mutateParams(params) {
  const next = { ...params };
  Object.keys(PARAM_BOUNDS).forEach((key) => {
    if (Math.random() > 0.7) return;
    const [lo, hi] = PARAM_BOUNDS[key];
    const scale = MUTATION_SCALE[key];
    const mutated = (next[key] ?? params[key]) + randNormal() * scale;
    next[key] = key === 'forwardPassMinGain' || key === 'markDistance'
      ? Math.round(clamp(mutated, lo, hi))
      : +clamp(mutated, lo, hi).toFixed(4);
  });
  return next;
}

function evaluatePolicies(baselineParams, candidateParams, games) {
  const totals = {
    baselineGoals: 0,
    candidateGoals: 0,
    passes: 0,
    passCompleted: 0,
    shots: 0,
    goals: 0,
    tackles: 0,
    tackleSuccess: 0,
    outOfBounds: 0,
  };

  const started = Date.now();

  for (let i = 0; i < games; i++) {
    const game = Engine.createGame({ aiOnly: true });
    game.aiPolicies[0] = { ...baselineParams };
    game.aiPolicies[1] = { ...baselineParams };

    while (game.phase !== 'fulltime') {
      Engine.stepGame(game, null, {
        teamPolicies: { 0: 'baseline', 1: 'candidate' },
        candidatePolicy: { parameters: candidateParams },
      });
    }

    totals.baselineGoals += game.score[0];
    totals.candidateGoals += game.score[1];
    totals.passes += game.stats.passes;
    totals.passCompleted += game.stats.passCompleted;
    totals.shots += game.stats.shots;
    totals.goals += game.stats.goals;
    totals.tackles += game.stats.tackles;
    totals.tackleSuccess += game.stats.tackleSuccess;
    totals.outOfBounds += game.stats.outOfBounds;
  }

  const elapsedMs = Date.now() - started;
  const goalDiff = totals.candidateGoals - totals.baselineGoals;
  return {
    games,
    elapsedMs,
    gamesPerSecond: +(games / (elapsedMs / 1000 || 1)).toFixed(2),
    baselineAvgGoals: +(totals.baselineGoals / games).toFixed(3),
    candidateAvgGoals: +(totals.candidateGoals / games).toFixed(3),
    goalDiff: +goalDiff.toFixed(3),
    candidateWon: goalDiff > 0,
    avgPasses: +(totals.passes / games).toFixed(2),
    passCompletionRate: +(totals.passes ? totals.passCompleted / totals.passes : 0).toFixed(3),
    avgShots: +(totals.shots / games).toFixed(2),
    avgGoals: +(totals.goals / games).toFixed(2),
    avgTackles: +(totals.tackles / games).toFixed(2),
    tackleSuccessRate: +(totals.tackles ? totals.tackleSuccess / totals.tackles : 0).toFixed(3),
    avgOutOfBounds: +(totals.outOfBounds / games).toFixed(2),
  };
}

function svgPolyline(points, xFor, yFor) {
  return points.map((p) => `${xFor(p.epoch).toFixed(1)},${yFor(p).toFixed(1)}`).join(' ');
}

function writeTrainingSvg(file, history, finalChampionEpoch) {
  const width = 1100;
  const height = 620;
  const pad = { left: 72, right: 32, top: 54, bottom: 78 };
  const plotW = width - pad.left - pad.right;
  const plotH = height - pad.top - pad.bottom;
  const epochs = Math.max(1, history.length);
  const goalValues = history.flatMap(h => [h.baselineAvgGoals, h.candidateAvgGoals, h.goalDiff]);
  const minY = Math.min(-1, ...goalValues);
  const maxY = Math.max(3, ...goalValues);
  const ySpan = maxY - minY || 1;
  const xFor = epoch => pad.left + ((epoch - 1) / Math.max(1, epochs - 1)) * plotW;
  const yForValue = value => pad.top + (1 - (value - minY) / ySpan) * plotH;
  const lineFor = key => svgPolyline(history, xFor, h => yForValue(h[key]));
  const accepted = history.filter(h => h.accepted);
  const grid = [];
  for (let i = 0; i <= 5; i++) {
    const value = minY + (ySpan * i / 5);
    const y = yForValue(value);
    grid.push(`<line x1="${pad.left}" y1="${y.toFixed(1)}" x2="${(width-pad.right)}" y2="${y.toFixed(1)}" stroke="#d8dee9" stroke-width="1" opacity="0.55"/>`);
    grid.push(`<text x="${pad.left-12}" y="${(y+4).toFixed(1)}" text-anchor="end" font-size="12" fill="#4b5563">${value.toFixed(1)}</text>`);
  }
  const xTicks = [];
  const tickStep = Math.max(1, Math.round(epochs / 10));
  for (let epoch = 1; epoch <= epochs; epoch += tickStep) {
    const x = xFor(epoch);
    xTicks.push(`<line x1="${x.toFixed(1)}" y1="${pad.top}" x2="${x.toFixed(1)}" y2="${height-pad.bottom}" stroke="#eef2f7" stroke-width="1"/>`);
    xTicks.push(`<text x="${x.toFixed(1)}" y="${height-pad.bottom+24}" text-anchor="middle" font-size="12" fill="#4b5563">${epoch}</text>`);
  }

  const acceptedDots = accepted.map(h => {
    const x = xFor(h.epoch);
    const y = yForValue(h.goalDiff);
    return `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="5" fill="#16a34a"><title>Accepted epoch ${h.epoch}, diff ${h.goalDiff}</title></circle>`;
  }).join('\n');

  const svg = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
  <rect width="100%" height="100%" fill="#ffffff"/>
  <text x="${pad.left}" y="30" font-family="Arial, sans-serif" font-size="22" font-weight="700" fill="#111827">Training progress</text>
  <text x="${width-pad.right}" y="30" text-anchor="end" font-family="Arial, sans-serif" font-size="13" fill="#4b5563">Final champion epoch: ${finalChampionEpoch}</text>
  <g font-family="Arial, sans-serif">
    ${xTicks.join('\n    ')}
    ${grid.join('\n    ')}
    <line x1="${pad.left}" y1="${height-pad.bottom}" x2="${width-pad.right}" y2="${height-pad.bottom}" stroke="#111827" stroke-width="1.4"/>
    <line x1="${pad.left}" y1="${pad.top}" x2="${pad.left}" y2="${height-pad.bottom}" stroke="#111827" stroke-width="1.4"/>
    <text x="${pad.left + plotW/2}" y="${height-24}" text-anchor="middle" font-size="13" fill="#111827">Epoch</text>
    <text x="20" y="${pad.top + plotH/2}" transform="rotate(-90 20 ${pad.top + plotH/2})" text-anchor="middle" font-size="13" fill="#111827">Goals / goal diff</text>
    <polyline fill="none" stroke="#2563eb" stroke-width="2.4" points="${lineFor('candidateAvgGoals')}"/>
    <polyline fill="none" stroke="#ef4444" stroke-width="2.4" points="${lineFor('baselineAvgGoals')}"/>
    <polyline fill="none" stroke="#7c3aed" stroke-width="2" stroke-dasharray="6 5" points="${lineFor('goalDiff')}"/>
    ${acceptedDots}
    <g transform="translate(${pad.left}, ${height-54})" font-size="13" fill="#111827">
      <rect x="0" y="-12" width="14" height="4" fill="#2563eb"/><text x="22" y="-7">Candidate avg goals</text>
      <rect x="190" y="-12" width="14" height="4" fill="#ef4444"/><text x="212" y="-7">Baseline avg goals</text>
      <rect x="378" y="-12" width="14" height="4" fill="#7c3aed"/><text x="400" y="-7">Goal diff</text>
      <circle cx="548" cy="-10" r="5" fill="#16a34a"/><text x="562" y="-7">Accepted improvement</text>
    </g>
  </g>
</svg>
`;
  fs.writeFileSync(file, svg);
}

function main() {
  const initial = readJson(BASELINE_PATH);
  const initialParams = { ...initial.parameters };
  let championParams = { ...initial.parameters };
  let championEpoch = 0;
  const sessionStarted = new Date().toISOString();
  const summary = [];

  fs.mkdirSync(SESSION_DIR, { recursive: true });

  writeJson(path.join(SESSION_DIR, 'epoch-000-baseline.json'), {
    name: 'epoch-000-baseline',
    session: SESSION_NAME,
    epoch: 0,
    createdAt: sessionStarted,
    source: 'data/policies/baseline.json',
    role: 'initial-baseline',
    parameters: championParams,
  });

  for (let epoch = 1; epoch <= EPOCHS; epoch++) {
    const opponentEpoch = championEpoch;
    const opponentParams = { ...championParams };
    const candidateParams = mutateParams(championParams);
    const evaluation = evaluatePolicies(championParams, candidateParams, GAMES_PER_EPOCH);
    const accepted = evaluation.candidateWon;
    if (accepted) {
      championParams = candidateParams;
      championEpoch = epoch;
    }

    const record = {
      name: `epoch-${String(epoch).padStart(3, '0')}`,
      session: SESSION_NAME,
      epoch,
      createdAt: new Date().toISOString(),
      gamesPerEpoch: GAMES_PER_EPOCH,
      opponent: {
        sourceEpoch: opponentEpoch,
        parameters: opponentParams,
      },
      candidate: {
        parameters: candidateParams,
      },
      accepted,
      championEpoch: accepted ? epoch : championEpoch,
      championParameters: accepted ? candidateParams : championParams,
      evaluation,
    };

    summary.push({
      epoch,
      accepted,
      championEpoch: record.championEpoch,
      goalDiff: evaluation.goalDiff,
      baselineAvgGoals: evaluation.baselineAvgGoals,
      candidateAvgGoals: evaluation.candidateAvgGoals,
      elapsedMs: evaluation.elapsedMs,
      championParameters: record.championParameters,
    });

    writeJson(path.join(SESSION_DIR, `epoch-${String(epoch).padStart(3, '0')}.json`), record);
    console.log(`${record.name} ${accepted ? 'ACCEPTED' : 'rejected'} diff=${evaluation.goalDiff} candidate=${evaluation.candidateAvgGoals} baseline=${evaluation.baselineAvgGoals} champion=${record.championEpoch}`);
  }

  writeJson(path.join(SESSION_DIR, 'summary.json'), {
    name: SESSION_NAME,
    startedAt: sessionStarted,
    finishedAt: new Date().toISOString(),
    epochs: EPOCHS,
    gamesPerEpoch: GAMES_PER_EPOCH,
    finalChampionEpoch: championEpoch,
    finalChampionParameters: championParams,
    acceptedEpochs: summary.filter(s => s.accepted).map(s => s.epoch),
    acceptedCount: summary.filter(s => s.accepted).length,
    rejectedCount: summary.filter(s => !s.accepted).length,
    bestGoalDiff: Math.max(...summary.map(s => s.goalDiff)),
    averageEpochElapsedMs: Math.round(summary.reduce((sum, s) => sum + s.elapsedMs, 0) / Math.max(1, summary.length)),
    totalTrainingElapsedMs: summary.reduce((sum, s) => sum + s.elapsedMs, 0),
    history: summary.map(({ championParameters, ...rest }) => rest),
  });

  writeJson(path.join(SESSION_DIR, 'best.json'), {
    name: `${SESSION_NAME}-best`,
    version: 1,
    type: 'rule-policy',
    sourceSession: SESSION_NAME,
    sourceEpoch: championEpoch,
    parameters: championParams,
  });

  writeTrainingSvg(path.join(SESSION_DIR, 'training-progress.svg'), summary, championEpoch);

  if (championEpoch > 0) {
    console.log('\nEvaluating final champion against original baseline...');
    const finalEval = evaluatePolicies(initialParams, championParams, GAMES_PER_EPOCH);
    console.log(`Final eval: champion=${finalEval.candidateAvgGoals} baseline=${finalEval.baselineAvgGoals} diff=${finalEval.goalDiff}`);
    if (finalEval.candidateWon) {
      writeJson(BASELINE_PATH, {
        ...initial,
        parameters: championParams,
        updatedAt: new Date().toISOString(),
        updatedBySession: SESSION_NAME,
        updatedByEpoch: championEpoch,
        finalEvalGoalDiff: finalEval.goalDiff,
      });
      console.log(`baseline.json updated with champion from epoch ${championEpoch} (diff=${finalEval.goalDiff})`);
    } else {
      console.log('Champion did not beat original baseline — baseline.json unchanged.');
    }
  } else {
    console.log('\nNo improvement found this session — baseline.json unchanged.');
  }
}

main();
