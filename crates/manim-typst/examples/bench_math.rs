//! Baseline latency of the current typst math pipeline.
//! Run: cargo run -p manim-typst --release --example bench_math

use std::time::Instant;

use manim_typst::{MathOptions, math_mobjects, tex_mobjects};

fn main() {
    let opts = MathOptions::default();
    let quad = "x = (-b plus.minus sqrt(b^2 - 4 a c)) / (2 a)";

    // 1. Cold: first call pays font loading (typst-assets) + comemo warmup.
    let t = Instant::now();
    let m = math_mobjects(quad, &opts).unwrap();
    println!("cold first call: {:?}  ({} mobjects)", t.elapsed(), m.len());

    // 2. Cache hit: identical source is memoized.
    let n_hit = 1000u32;
    let t = Instant::now();
    for _ in 0..n_hit {
        std::hint::black_box(math_mobjects(quad, &opts).unwrap());
    }
    println!("cache hit:       {:?}/call", t.elapsed() / n_hit);

    // 3. Warm unique compiles (suffix bypasses the memo cache).
    let equations = [
        ("quadratic", quad),
        ("sum", "sum_(n=1)^infinity 1/n^2 = pi^2 / 6"),
        ("integral", "integral_0^infinity e^(-x^2) d x = sqrt(pi) / 2"),
        ("matrix", "A = mat(1, 2; 3, 4), quad B = mat(a, b; c, d)"),
        (
            "mixed",
            "e^(i pi) + 1 = 0, quad nabla^2 psi = 0, quad lim_(x -> 0) sin(x)/x = 1",
        ),
    ];
    let n = 200u32;
    for (name, eq) in equations {
        let _ = math_mobjects(&format!("{eq} + z_0"), &opts).unwrap();
        let t = Instant::now();
        for i in 1..=n {
            std::hint::black_box(math_mobjects(&format!("{eq} + z_{i}"), &opts).unwrap());
        }
        println!("{name:10} {:?}/eq  (warm, unique source)", t.elapsed() / n);
    }

    // 4. LaTeX input: mitex conversion + shim prelude on top of the same
    // memoized compile path. Expect a small constant overhead per unique
    // source and identical cache-hit behavior.
    let latex_equations = [
        ("tex_quad", r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"),
        ("tex_basel", r"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}"),
        ("tex_pmatrix", r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}"),
    ];
    for (name, eq) in latex_equations {
        // Cache hit first (conversion is memoized with the compile).
        let t = Instant::now();
        for _ in 0..n_hit {
            std::hint::black_box(tex_mobjects(eq, &opts).unwrap());
        }
        println!("{name:10} {:?}/call  (cache hit)", t.elapsed() / n_hit);

        let _ = tex_mobjects(&format!("{eq} + z_0"), &opts).unwrap();
        let t = Instant::now();
        for i in 1..=n {
            std::hint::black_box(tex_mobjects(&format!("{eq} + z_{i}"), &opts).unwrap());
        }
        println!("{name:10} {:?}/eq   (warm, unique source)", t.elapsed() / n);
    }
}
