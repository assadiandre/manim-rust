use manim_core::kurbo::Shape;
use manim_typst::{math_mobjects, tex_mobjects, MathOptions};

#[test]
fn euler_identity_produces_glyph_paths() {
    let parts = math_mobjects("e^{i pi} + 1 = 0", &MathOptions::default()).unwrap();
    assert!(
        parts.len() > 5,
        "expected many glyph fragments, got {}",
        parts.len()
    );
    for p in &parts {
        assert!(!p.path.elements().is_empty());
        assert!(p.style.fill.is_some(), "glyphs should be filled");
    }
}

#[test]
fn fraction_and_sqrt_have_shape_fragments() {
    // Fractions/radicals emit stroked Shape items (vinculum, radical
    // overline) in addition to glyph fills.
    let parts = math_mobjects("frac(a, b) + sqrt(x)", &MathOptions::default()).unwrap();
    assert!(parts.len() >= 7, "got {}", parts.len());
    let stroked = parts.iter().filter(|p| p.style.stroke.is_some()).count();
    assert!(stroked >= 2, "expected vinculum + radical overline, got {stroked}");
}

#[test]
fn compile_is_fast_and_cached() {
    let opts = MathOptions::default();
    // Warm the font/library statics with a different formula.
    math_mobjects("x = 1", &opts).unwrap();

    let start = std::time::Instant::now();
    math_mobjects("sum_(n=1)^infinity 1/n^2 = pi^2/6", &opts).unwrap();
    let cold = start.elapsed();
    assert!(cold.as_secs_f64() < 2.0, "cold compile took {cold:?}");

    let start = std::time::Instant::now();
    math_mobjects("sum_(n=1)^infinity 1/n^2 = pi^2/6", &opts).unwrap();
    let cached = start.elapsed();
    assert!(cached.as_millis() < 5, "cached compile took {cached:?}");
}

#[test]
fn invalid_math_reports_error() {
    let result = math_mobjects("frac(", &MathOptions::default());
    assert!(result.is_err());
}

#[test]
fn explicit_color_overrides_black_to_white_mapping() {
    use manim_core::peniko::Color;
    let red = Color::from_rgba8(255, 0, 0, 255);
    let opts = MathOptions {
        color: Some(red),
        ..Default::default()
    };
    let parts = math_mobjects("x^2", &opts).unwrap();
    assert!(!parts.is_empty());
    for p in &parts {
        let c = p.style.fill.unwrap().to_rgba8();
        assert_eq!((c.r, c.g, c.b), (255, 0, 0));
    }
}

// ---------------------------------------------------------------------------
// LaTeX input (tex_mobjects, via mitex)

#[test]
fn tex_euler_identity_produces_glyph_paths() {
    let parts = tex_mobjects(r"e^{i\pi} + 1 = 0", &MathOptions::default()).unwrap();
    assert!(
        parts.len() > 5,
        "expected many glyph fragments, got {}",
        parts.len()
    );
    for p in &parts {
        assert!(!p.path.elements().is_empty());
        assert!(p.style.fill.is_some(), "glyphs should be filled");
    }
}

#[test]
fn tex_fraction_and_sqrt_have_shape_fragments() {
    let parts = tex_mobjects(r"\frac{a}{b} + \sqrt{x}", &MathOptions::default()).unwrap();
    assert!(parts.len() >= 7, "got {}", parts.len());
    let stroked = parts.iter().filter(|p| p.style.stroke.is_some()).count();
    assert!(
        stroked >= 2,
        "expected vinculum + radical overline, got {stroked}"
    );
}

#[test]
fn tex_invalid_math_reports_error() {
    let result = tex_mobjects(r"\frac{", &MathOptions::default());
    assert!(result.is_err());
}

#[test]
fn tex_and_typst_fractions_have_same_geometry() {
    // Not identical paths (different source strings), but the same typeset
    // geometry: mitex's `frac(a ,b )` must match native `frac(a, b)`.
    let opts = MathOptions::default();
    let bbox = |parts: Vec<manim_core::Mobject>| {
        parts
            .iter()
            .map(|p| p.path.bounding_box())
            .reduce(|a, b| a.union(b))
            .unwrap()
    };
    let tex = bbox(tex_mobjects(r"\frac{a}{b}", &opts).unwrap());
    let typ = bbox(math_mobjects("frac(a, b)", &opts).unwrap());
    for (t, p) in [
        (tex.width(), typ.width()),
        (tex.height(), typ.height()),
        (tex.center().x, typ.center().x),
        (tex.center().y, typ.center().y),
    ] {
        assert!(
            (t - p).abs() < 1e-6,
            "bbox mismatch: tex {tex:?} vs typst {typ:?}"
        );
    }
}

#[test]
fn tex_textcolor_applies_via_shim() {
    // The mitex color shim extracts the color name from math-letter content;
    // a silent failure would fall back to white, so assert the actual color.
    let parts = tex_mobjects(r"\textcolor{blue}{x}", &MathOptions::default()).unwrap();
    assert!(!parts.is_empty());
    for p in &parts {
        let c = p.style.fill.unwrap().to_rgba8();
        assert_eq!((c.r, c.g, c.b), (0, 0, 255), "expected pure blue, got {c:?}");
    }
}
