//! LaTeX corpus regression test: every construct below must survive the
//! mitex -> MITEX_PRELUDE -> typst pipeline end to end. Prints conversions
//! for eyeballing; run with -- --nocapture.

use manim_typst::{tex_mobjects, MathOptions};

const CORPUS: [(&str, &str); 19] = [
    ("euler", r"e^{i\pi}+1=0"),
    ("frac_sqrt", r"\frac{a}{b}+\sqrt{x}"),
    ("basel", r"\sum_{n=1}^{\infty} \frac{1}{n^2}=\frac{\pi^2}{6}"),
    ("pmatrix", r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}"),
    (
        "fourier",
        r"\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x) e^{-i 2\pi \xi x} dx",
    ),
    // trickier constructs from the probe
    ("sqrt_idx", r"\sqrt[3]{x}"),
    ("color", r"\color{red} x + y"),
    ("textcolor", r"\textcolor{blue}{x}"),
    ("text", r"\text{hello world}"),
    ("overbrace", r"\overbrace{a+b}^{k}"),
    ("underbrace", r"\underbrace{x \times y}_{n}"),
    ("opname", r"\operatorname*{arg\,max}_{x}"),
    ("aligned", r"\begin{aligned} a &= b \\ c &= d \end{aligned}"),
    ("cases", r"\begin{cases} a & b \\ c & d \end{cases}"),
    ("displaystyle", r"\displaystyle\sum_{n=1}^{\infty}"),
    ("mathbf", r"\mathbf{v} + \mathit{w}"),
    ("left_right", r"\left( \frac{a}{b} \right)"),
    ("notin", r"x \notin S"),
    ("boxed", r"\boxed{E = mc^2}"),
];

#[test]
fn mitex_corpus_converts_and_compiles() {
    let opts = MathOptions::default();
    let mut failures = Vec::new();
    for (name, latex) in CORPUS {
        match mitex::convert_math(latex, None) {
            Err(e) => {
                println!("{name:12} CONVERT ERR: {e}");
                failures.push(name);
            }
            Ok(typ) => {
                println!("{name:12} -> {typ}");
                match tex_mobjects(latex, &opts) {
                    Err(e) => {
                        println!("{name:12} COMPILE ERR: {e}");
                        failures.push(name);
                    }
                    Ok(parts) => println!("{name:12} ok: {} mobjects", parts.len()),
                }
            }
        }
    }
    assert!(failures.is_empty(), "failed: {failures:?}");
}
