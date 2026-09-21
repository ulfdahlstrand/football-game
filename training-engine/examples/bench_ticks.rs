//! Tillfällig mätning: matcher/sek och antal spelarbeslut per match.
use std::sync::atomic::{AtomicU64, Ordering};
use rand::SeedableRng;
use rand::rngs::SmallRng;
use training_engine::game::{Game, Phase};
use training_engine::physics::step_game;
use training_engine::policy::v6_default_for_slot;
use training_engine::policy::TeamPolicyV6;
use training_engine::team::Team;
use training_engine::team_v6::V6Team;

static DECISIONS: AtomicU64 = AtomicU64::new(0);

struct Counting(V6Team);
impl Team for Counting {
    fn pre_tick(&mut self, g: &Game) { self.0.pre_tick(g) }
    fn tick_player(&mut self, g: &mut Game, i: usize, rng: &mut dyn rand::RngCore) {
        DECISIONS.fetch_add(1, Ordering::Relaxed);
        self.0.tick_player(g, i, rng)
    }
    fn team_id(&self) -> usize { self.0.team_id() }
}

fn policy() -> TeamPolicyV6 { std::array::from_fn(v6_default_for_slot) }

fn main() {
    let p = policy();
    // 1) en match, räkna beslut + ticks
    let mut rng = SmallRng::seed_from_u64(42);
    let mut game = Game::new();
    let mut teams: [Box<dyn Team>; 2] = [
        Box::new(Counting(V6Team::new(0, p))),
        Box::new(Counting(V6Team::new(1, p))),
    ];
    let mut ticks = 0u64;
    let t0 = std::time::Instant::now();
    while game.phase != Phase::Fulltime { step_game(&mut game, &mut teams, &mut rng); ticks += 1; }
    let one = t0.elapsed();
    println!("ticks/match            : {}", ticks);
    println!("spelarbeslut/match     : {}", DECISIONS.load(Ordering::Relaxed));
    println!("tid 1 match (1 tråd)   : {:.2} ms", one.as_secs_f64() * 1000.0);

    // 2) seriell genomströmning
    let n = 200;
    let t1 = std::time::Instant::now();
    for s in 0..n {
        let mut rng = SmallRng::seed_from_u64(s);
        let mut g = Game::new();
        let mut t: [Box<dyn Team>; 2] = [Box::new(V6Team::new(0, p)), Box::new(V6Team::new(1, p))];
        while g.phase != Phase::Fulltime { step_game(&mut g, &mut t, &mut rng); }
    }
    let ser = t1.elapsed();
    println!("seriellt               : {:.0} matcher/s ({:.2} ms/match)",
        n as f64 / ser.as_secs_f64(), ser.as_secs_f64() * 1000.0 / n as f64);

    // 3) parallellt (som trainern)
    use rayon::prelude::*;
    let n2 = 2000;
    let t2 = std::time::Instant::now();
    (0..n2).into_par_iter().for_each(|s| {
        let mut rng = SmallRng::seed_from_u64(s as u64);
        let mut g = Game::new();
        let mut t: [Box<dyn Team>; 2] = [Box::new(V6Team::new(0, p)), Box::new(V6Team::new(1, p))];
        while g.phase != Phase::Fulltime { step_game(&mut g, &mut t, &mut rng); }
    });
    let par = t2.elapsed();
    println!("parallellt ({} kärnor) : {:.0} matcher/s", rayon::current_num_threads(),
        n2 as f64 / par.as_secs_f64());
    println!("=> spelarbeslut/s      : {:.2e}",
        (n2 as f64 / par.as_secs_f64()) * DECISIONS.load(Ordering::Relaxed) as f64);
}
