//! Shared scene definitions used by the CLI (`demo`, `png`, `visual-check`)
//! and by golden tests. Keeping them here means the thing a human reviews in
//! a contact sheet is bit-identical to the thing CI compares against goldens.

use kurbo::{Affine, Point, Vec2};
use manim_anim::{Animation, Scene};
use manim_core::constants::{LEFT, RIGHT};
use manim_core::{
    add_arrow, add_axes, geometry, palette, AxesOpts, Mobject, Style,
};
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

    // layout: next_to / arrange, static.
    let mut s = Scene::new();
    let c = s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 0.8))
            .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 4.0)),
    );
    let sq = s.add(
        Mobject::new(geometry::square(Point::ORIGIN, 1.2))
            .with_style(Style::filled(palette::red()).with_stroke(palette::white(), 4.0)),
    );
    let tri = s.add(
        Mobject::new(geometry::triangle(Point::ORIGIN, 0.8))
            .with_style(Style::filled(palette::green()).with_stroke(palette::white(), 4.0)),
    );
    s.graph.next_to(sq, c, LEFT, 0.3);
    s.graph.next_to(tri, c, RIGHT, 0.3);
    out.push(probe("layout", s, &[0.0]));

    // geometry gallery: the new primitives, static.
    let mut s = Scene::new();
    s.add(
        Mobject::new(geometry::ellipse(Point::new(-4.0, 2.0), 1.2, 0.6))
            .with_style(Style::filled(palette::teal()).with_stroke(palette::white(), 4.0)),
    );
    s.add(
        Mobject::new(geometry::arc(Point::new(-1.5, 2.0), 0.9, 0.3, 4.0))
            .with_style(Style::default().with_stroke(palette::gold(), 6.0).no_fill()),
    );
    s.add(
        Mobject::new(geometry::sector(Point::new(1.5, 2.0), 0.9, 0.4, 2.0))
            .with_style(Style::filled(palette::orange()).with_stroke(palette::white(), 3.0)),
    );
    s.add(
        Mobject::new(geometry::annulus(Point::new(4.0, 2.0), 0.4, 0.9))
            .with_style(Style::filled(palette::purple()).no_stroke()),
    );
    s.add(
        Mobject::new(geometry::rounded_rect(Point::new(-4.0, -1.5), 2.0, 1.2, 0.3))
            .with_style(Style::filled(palette::maroon()).with_stroke(palette::white(), 4.0)),
    );
    s.add(
        Mobject::new(geometry::dashed_line(
            Point::new(-2.2, -1.5),
            Point::new(0.4, -0.6),
            0.18,
            0.1,
        ))
        .with_style(Style::default().with_stroke(palette::yellow(), 5.0).no_fill()),
    );
    add_arrow(
        &mut s.graph,
        Point::new(1.0, -2.0),
        Point::new(4.2, -0.6),
        0.0,
        0.35,
        Style::default().with_stroke(palette::gold(), 6.0),
    );
    out.push(probe("geometry", s, &[0.0]));

    // rotate a triangle a quarter turn (a square would look unchanged at 90°).
    let mut s = Scene::new();
    let tri = s.add(
        Mobject::new(geometry::triangle(Point::ORIGIN, 1.6))
            .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 4.0)),
    );
    s.play([Animation::rotate(&s.graph, tri, std::f64::consts::FRAC_PI_2, 1.0)]);
    out.push(probe("rotate", s, &[0.0, 0.5, 1.0]));

    // uncreate a circle.
    let mut s = Scene::new();
    let c = s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 1.5))
            .with_style(Style::filled(palette::red()).with_stroke(palette::white(), 4.0)),
    );
    s.play([Animation::uncreate(&s.graph, c, 1.0)]);
    out.push(probe("uncreate", s, &[0.0, 0.5, 1.0]));

    // grow from center.
    let mut s = Scene::new();
    let c = s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 1.5))
            .with_style(Style::filled(palette::green()).with_stroke(palette::white(), 4.0)),
    );
    s.play([Animation::grow_from_center(&s.graph, c, 1.0)]);
    out.push(probe("grow", s, &[0.0, 0.5, 1.0]));

    // indicate pulse.
    let mut s = Scene::new();
    let c = s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 1.2))
            .with_style(Style::filled(palette::yellow()).with_stroke(palette::white(), 4.0)),
    );
    s.play([Animation::indicate(&s.graph, c, 1.0)]);
    out.push(probe("indicate", s, &[0.0, 0.5, 1.0]));

    // write a formula glyph-by-glyph.
    let mut s = Scene::new();
    let tex = add_math(&mut s.graph, "e^{i pi} + 1 = 0", &MathOptions::default())
        .expect("typst compile failed");
    s.play_write(tex, 1.5);
    out.push(probe("write", s, &[0.0, 0.4, 0.8, 1.2, 1.5]));

    // axes + a baked parabola.
    let mut s = Scene::new();
    add_axes(
        &mut s.graph,
        &AxesOpts {
            x_min: -3.0,
            x_max: 3.0,
            y_min: -1.0,
            y_max: 3.0,
            unit_size: 1.0,
            ..AxesOpts::default()
        },
        Style::default().with_stroke(palette::gray(), 3.0),
    );
    s.add(
        Mobject::new(geometry::plot(-2.2, 2.2, 80, 1.0, 1.0, |x| 0.35 * x * x))
            .with_style(Style::default().with_stroke(palette::yellow(), 5.0).no_fill()),
    );
    out.push(probe("axes", s, &[0.0]));

    out
}
