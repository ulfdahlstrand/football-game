const Engine = require('./match-engine');
const fs = require('fs');

const games = Number(process.argv[2] || 10);
const mode = process.argv[3] || 'baseline-vs-baseline';
const candidatePolicy = JSON.parse(fs.readFileSync('data/policies/candidate.json', 'utf8'));
const teamPolicies = mode === 'candidate-vs-baseline'
  ? { 0: 'candidate', 1: 'baseline' }
  : mode === 'baseline-vs-candidate'
    ? { 0: 'baseline', 1: 'candidate' }
    : { 0: 'baseline', 1: 'baseline' };
const started = Date.now();
const totals = {
  home: 0,
  away: 0,
  passes: 0,
  passCompleted: 0,
  shots: 0,
  goals: 0,
  tackles: 0,
  tackleSuccess: 0,
  outOfBounds: 0,
};

for (let i = 0; i < games; i++) {
  const game = Engine.runSimulation({ aiOnly: true, teamPolicies, candidatePolicy });
  totals.home += game.score[0];
  totals.away += game.score[1];
  totals.passes += game.stats.passes;
  totals.passCompleted += game.stats.passCompleted;
  totals.shots += game.stats.shots;
  totals.goals += game.stats.goals;
  totals.tackles += game.stats.tackles;
  totals.tackleSuccess += game.stats.tackleSuccess;
  totals.outOfBounds += game.stats.outOfBounds;
}

const elapsed = Date.now() - started;
const passRate = totals.passes ? totals.passCompleted / totals.passes : 0;
const tackleRate = totals.tackles ? totals.tackleSuccess / totals.tackles : 0;

console.log(JSON.stringify({
  games,
  mode,
  elapsedMs: elapsed,
  gamesPerSecond: +(games / (elapsed / 1000 || 1)).toFixed(2),
  avgScore: {
    home: +(totals.home / games).toFixed(2),
    away: +(totals.away / games).toFixed(2),
  },
  avgPasses: +(totals.passes / games).toFixed(2),
  passCompletionRate: +passRate.toFixed(3),
  avgShots: +(totals.shots / games).toFixed(2),
  avgGoals: +(totals.goals / games).toFixed(2),
  avgTackles: +(totals.tackles / games).toFixed(2),
  tackleSuccessRate: +tackleRate.toFixed(3),
  avgOutOfBounds: +(totals.outOfBounds / games).toFixed(2),
}, null, 2));
