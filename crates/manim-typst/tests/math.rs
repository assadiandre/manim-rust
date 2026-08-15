use manim_core::kurbo::Shape;
use manim_typst::{math_mobjects, tex_mobjects, text_mobjects, MathOptions};

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
    assert!(
        stroked >= 2,
        "expected vinculum + radical overline, got {stroked}"
    );
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
fn title_sits_on_the_top_edge() {
    use manim_core::constants::{FRAME_Y_RADIUS, UP};
    use manim_core::SceneGraph;
    use manim_typst::add_title;
    let mut g = SceneGraph::new();
    let id = add_title(&mut g, "Title", &MathOptions::default()).unwrap();
    let bb = g.bounding_box(id);
    let top = g.critical_point(id, UP);
    assert!(
        (top.y - (FRAME_Y_RADIUS - 0.4)).abs() < 0.15,
        "title top y={}",
        top.y
    );
    assert!(
        bb.center().x.abs() < 0.5,
        "title should stay horizontally centered, bbox={bb:?}"
    );
}

#[test]
fn plain_text_produces_glyph_paths() {
    let parts = text_mobjects("Hello", &MathOptions::default()).unwrap();
    assert!(!parts.is_empty(), "expected glyph fragments");
    for p in &parts {
        assert!(!p.path.elements().is_empty());
        assert!(p.style.fill.is_some(), "text glyphs should be filled");
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
        assert_eq!(
            (c.r, c.g, c.b),
            (0, 0, 255),
            "expected pure blue, got {c:?}"
        );
    }
}

#[test]
fn table_two_by_two_has_four_cells() {
    use manim_core::SceneGraph;
    use manim_typst::add_table;
    let mut g = SceneGraph::new();
    let id = add_table(
        &mut g,
        &[vec!["a".into(), "b".into()], vec!["c".into(), "d".into()]],
        &MathOptions::default(),
    )
    .unwrap();
    assert_eq!(g.children_of(id).len(), 4);
}

#[test]
fn matrix_two_by_two_has_glyphs() {
    use manim_core::SceneGraph;
    use manim_typst::add_matrix;
    let mut g = SceneGraph::new();
    let id = add_matrix(
        &mut g,
        &[vec![1.0, 2.0], vec![3.0, 4.0]],
        &MathOptions::default(),
    )
    .unwrap();
    let n = g.children_of(id).len();
    assert!(n > 4, "expected several path children, got {n}");
}

#[test]
fn code_hello_has_glyphs() {
    use manim_core::SceneGraph;
    use manim_typst::add_code;
    let mut g = SceneGraph::new();
    let id = add_code(&mut g, "fn main() {}", &MathOptions::default()).unwrap();
    assert!(
        !g.children_of(id).is_empty(),
        "expected code glyphs from #raw"
    );
}

#[test]
fn number_line_labels_count() {
    use manim_core::SceneGraph;
    use manim_typst::add_number_line_labels;
    let mut g = SceneGraph::new();
    let id = add_number_line_labels(&mut g, -2.0, 2.0, 1.0, 1.0, false, &MathOptions::default())
        .unwrap();
    assert_eq!(g.children_of(id).len(), 5);
}

#[test]
fn digit_atlas_composes_two_digit_number() {
    use manim_typst::digit_atlas;
    let atlas = digit_atlas(&MathOptions::default()).unwrap();
    let one = atlas.compose(1.0, 0);
    let twelve = atlas.compose(12.0, 0);
    let w1 = one.bounding_box().width();
    let w12 = twelve.bounding_box().width();
    assert!(
        w12 > w1,
        "compose(12, 0) width {w12} should exceed compose(1, 0) {w1}"
    );
}

#[test]
fn graph_label_sits_right_of_yx_plot() {
    use manim_core::constants::RIGHT;
    use manim_core::geometry;
    use manim_core::{Mobject, SceneGraph};
    use manim_typst::add_graph_label;
    let mut g = SceneGraph::new();
    let plot = g.add(Mobject::new(geometry::plot(-2.0, 2.0, 17, 1.0, 1.0, |x| x)));
    let id = add_graph_label(
        &mut g,
        plot,
        "f(x)",
        1.0,
        RIGHT,
        0.25,
        &MathOptions::default(),
    )
    .unwrap();
    let left = g.critical_point(id, manim_core::constants::LEFT);
    let center = g.center_of(id);
    assert!(
        (left.x - 1.25).abs() < 1e-6,
        "label left x={} expected 1.25",
        left.x
    );
    assert!(
        (center.y - 1.0).abs() < 1e-6,
        "label center y={} expected 1.0",
        center.y
    );
    assert!(
        center.x > 1.0,
        "label center should be to the right of (1,1)"
    );
    assert_eq!(g.get(id).name.as_deref(), Some("graph_label"));
}

#[test]
fn complex_plane_labels_include_i() {
    use manim_core::SceneGraph;
    use manim_typst::add_complex_plane_labels;
    let mut g = SceneGraph::new();
    let id = add_complex_plane_labels(
        &mut g,
        -1.0,
        1.0,
        1.0,
        -1.0,
        1.0,
        1.0,
        1.0,
        false,
        &MathOptions::default(),
    )
    .unwrap();
    assert!(
        g.children_of(id).len() >= 4,
        "expected real ticks plus i/-i, got {}",
        g.children_of(id).len()
    );
}
