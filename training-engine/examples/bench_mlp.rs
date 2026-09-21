//! Mikromätning: kostnad för en liten MLP-forward pass per spelarbeslut.
use std::hint::black_box;

fn forward<const I: usize, const H: usize, const O: usize>(
    x: &[f32; I], w1: &[f32], b1: &[f32; H], w2: &[f32], b2: &[f32; O], out: &mut [f32; O],
) {
    let mut h = [0f32; H];
    for j in 0..H {
        let mut s = b1[j];
        for i in 0..I { s += w1[j * I + i] * x[i]; }
        h[j] = if s > 0.0 { s } else { 0.01 * s }; // leaky relu
    }
    for k in 0..O {
        let mut s = b2[k];
        for j in 0..H { s += w2[k * H + j] * h[j]; }
        out[k] = s.tanh();
    }
}

fn bench<const I: usize, const H: usize, const O: usize>(label: &str, iters: u64) {
    let w1: Vec<f32> = (0..H * I).map(|i| (i as f32).sin() * 0.1).collect();
    let w2: Vec<f32> = (0..O * H).map(|i| (i as f32).cos() * 0.1).collect();
    let b1 = [0.01f32; H];
    let b2 = [0.01f32; O];
    let mut x = [0.5f32; I];
    let mut out = [0f32; O];
    let t = std::time::Instant::now();
    for n in 0..iters {
        x[0] = (n % 100) as f32 * 0.01;
        forward::<I, H, O>(black_box(&x), &w1, &b1, &w2, &b2, &mut out);
        black_box(&out);
    }
    let e = t.elapsed();
    let ns = e.as_secs_f64() * 1e9 / iters as f64;
    println!("{label:28} vikter={:5}  {ns:7.1} ns/beslut  ({:.2e} beslut/s)",
        H * I + O * H + H + O, 1e9 / ns);
}

fn main() {
    let n = 5_000_000;
    bench::<20, 8, 13>("MLP 20-8-13 (minimal)", n);
    bench::<24, 16, 16>("MLP 24-16-16 (liten)", n);
    bench::<32, 32, 16>("MLP 32-32-16 (mellan)", n);
    bench::<48, 64, 20>("MLP 48-64-20 (stor)", n / 5);
    bench::<64, 128, 24>("MLP 64-128-24 (XL)", n / 20);
    println!("\nReferens: nuvarande v6_tick ≈ 385 ns/beslut (inkl. fysik & spatial-sök)");
}
