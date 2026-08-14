//! Shared scene definitions used by the CLI (`demo`, `png`, `visual-check`)
//! and by golden tests. Keeping them here means the thing a human reviews in
//! a contact sheet is bit-identical to the thing CI compares against goldens.

use kurbo::{Affine, Point, Vec2};
use manim_anim::{Animation, Scene};
use manim_core::{geometry, palette, Mobject, Style};
use manim_typst::{add_math, MathOptions};

/// The north-star scene: circle draws itself, morphs into a square, and a
/// formula (typeset in-process by typst) fades in above it.
pub fn demo(formula: &str) -> Scene {
    let mut scene = Scene::new();

    let circle = scene.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 1.5)).with_style(
            Style::filled(palette::blue()).with_stroke(palette::white(), 4.0),
        ),
    );
    scene.play([Animation::create(&scene.graph, circle, 1.0)]);

    let square = geometry::square(Point::ORIGIN, 3.0);
    scene.play([Animation::morph(&scene.graph, circle, square, 1.2)]);

    let tex = add_math(&mut scene.graph, formula, &MathOptions::default())
        .expect("typst compile failed");
    scene.graph.get_mut(tex).transform = Affine::translate((0.0, 2.6));
    scene.play([Animation::fade_in(&scene.graph, tex, 0.8)]);

    scene.wait(0.5);
    scene
}

/// A named scene plus the timestamps a reviewer should inspect.
pub struct Probe {
    pub name: String,
    pub scene: Scene,
    pub times: Vec<f64>,
}

fn probe(name: &str, scene: Scene, times: &[f64]) -> Probe {
    Probe {
        name: name.to_string(),
        scene,
        times: times.to_vec(),
    }
}

/// One probe per animation primitive plus tex samples and the demo itself.
/// Timestamps are chosen at start / midpoint / end of each effect.
pub fn probes() -> Vec<Probe> {
    let mut out = Vec::new();

    // The demo, at semantically meaningful instants.
    out.push(probe(
        "demo",
        demo("e^{i pi} + 1 = 0"),
        &[0.0, 0.5, 1.0, 1.6, 2.2, 2.6, 3.0, 3.45],
    ));

    // create: filled circle traces itself.
    let mut s = Scene::new();
    let c = s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 1.5)).with_style(
            Style::filled(palette::blue()).with_stroke(palette::white(), 4.0),
        ),
    );
    s.play([Animation::create(&s.graph, c, 1.0)]);
    out.push(probe("create", s, &[0.0, 0.25, 0.5, 0.75, 1.0]));

    // fade_in / fade_out.
    let mut s = Scene::new();
    let sq = s.add(
        Mobject::new(geometry::square(Point::ORIGIN, 2.5)).with_style(
            Style::filled(palette::green()).with_stroke(palette::white(), 4.0),
        ),
    );
    s.play([Animation::fade_in(&s.graph, sq, 0.8)]);
    s.play([Animation::fade_out(&s.graph, sq, 0.8)]);
    out.push(probe("fade", s, &[0.0, 0.4, 0.8, 1.2, 1.6]));

    // shift.
    let mut s = Scene::new();
    let c = s.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)).with_style(
        Style::filled(palette::red()).with_stroke(palette::white(), 4.0),
    ));
    s.play([Animation::shift(&s.graph, c, Vec2::new(3.0, 1.0), 1.0)]);
    out.push(probe("shift", s, &[0.0, 0.5, 1.0]));

    // scale (about center).
    let mut s = Scene::new();
    let c = s.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)).with_style(
        Style::filled(palette::yellow()).with_stroke(palette::white(), 4.0),
    ));
    s.play([Animation::scale(&s.graph, c, 2.0, 1.0)]);
    out.push(probe("scale", s, &[0.0, 0.5, 1.0]));

    // morph: circle -> triangle.
    let mut s = Scene::new();
    let c = s.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.5)).with_style(
        Style::filled(palette::blue()).with_stroke(palette::white(), 4.0),
    ));
    let mut tri = kurbo::BezPath::new();
    tri.move_to(Point::new(0.0, 1.5));
    tri.line_to(Point::new(-1.5, -1.0));
    tri.line_to(Point::new(1.5, -1.0));
    tri.close_path();
    s.play([Animation::morph(&s.graph, c, tri, 1.2)]);
    out.push(probe("morph", s, &[0.0, 0.3, 0.6, 0.9, 1.2]));

    // tex gallery: static, single frame each.
    for (i, f) in [
        "frac{a}{b} + sqrt{x}",
        "sum_(n=1)^infinity 1/n^2 = pi^2/6",
        "integral_0^1 x^2 dif x = 1/3",
        "alpha + beta = gamma",
    ]
    .iter()
    .enumerate()
    {
        let mut s = Scene::new();
        add_math(&mut s.graph, f, &MathOptions::default()).expect("typst compile failed");
        out.push(probe(&format!("tex_{i}"), s, &[0.0]));
    }

    out
}
