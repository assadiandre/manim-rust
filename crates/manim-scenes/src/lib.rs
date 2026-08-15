//! Shared scene definitions used by the CLI (`demo`, `png`, `visual-check`)
//! and by golden tests. Keeping them here means the thing a human reviews in
//! a contact sheet is bit-identical to the thing CI compares against goldens.

use kurbo::{Affine, Point, Vec2};
use manim_anim::{Animation, Scene};
use manim_core::constants::{DOWN, LEFT, RIGHT};
use manim_core::{
    add_angle, add_arc_polygon, add_area_between, add_area_under, add_arrow, add_arrow_field, add_axes, add_brace,
    add_complex_plane,
    add_boolean, add_curved_arrow, add_curved_double_arrow, add_dashed_copy, add_graph,
    add_implicit_curve, add_number_line, add_raster,
    add_svg, checkerboard, raster_from_rgba,
    add_number_plane, add_polar_plane, add_riemann_rects, add_right_angle, add_surrounding_rect,
    add_tangent_line, add_underline, add_vertical_line_to_graph, geometry, layout_graph, palette,
    AxesOpts, BooleanOp, Mobject, NumberLineOpts, NumberPlaneOpts, PolarPlaneOpts, RiemannSample,
    Style,
};
use manim_typst::{
    add_bar_chart_labeled, add_bulleted_list, add_code, add_complex_plane_labels, add_decimal,
    add_decimal_atlas, add_graph_label, add_highlighted_cell, add_labeled_arrow, add_labeled_dot,
    add_labeled_line, add_markup, add_math, add_math_table, add_matrix, add_number_line_labels,
    add_graph_labeled, add_paragraph, add_table, add_table_labeled, add_table_with_lines, add_tex_parts,
    add_text, add_title, set_color_by_tex,
    digit_atlas, MathOptions,
};

/// The north-star scene: circle draws itself, morphs into a square, and a
/// formula (typeset in-process by typst) fades in above it.
pub fn demo(formula: &str) -> Scene {
    let mut scene = Scene::new();

    let circle = scene.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 1.5))
            .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 4.0)),
    );
    scene.play([Animation::create(&scene.graph, circle, 1.0)]);

    let square = geometry::square(Point::ORIGIN, 3.0);
    scene.play([Animation::morph(&scene.graph, circle, square, 1.2)]);

    let tex =
        add_math(&mut scene.graph, formula, &MathOptions::default()).expect("typst compile failed");
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
        Mobject::new(geometry::circle(Point::ORIGIN, 1.5))
            .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 4.0)),
    );
    s.play([Animation::create(&s.graph, c, 1.0)]);
    out.push(probe("create", s, &[0.0, 0.25, 0.5, 0.75, 1.0]));

    // fade_in / fade_out.
    let mut s = Scene::new();
    let sq = s.add(
        Mobject::new(geometry::square(Point::ORIGIN, 2.5))
            .with_style(Style::filled(palette::green()).with_stroke(palette::white(), 4.0)),
    );
    s.play([Animation::fade_in(&s.graph, sq, 0.8)]);
    s.play([Animation::fade_out(&s.graph, sq, 0.8)]);
    out.push(probe("fade", s, &[0.0, 0.4, 0.8, 1.2, 1.6]));

    // shift.
    let mut s = Scene::new();
    let c = s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 1.0))
            .with_style(Style::filled(palette::red()).with_stroke(palette::white(), 4.0)),
    );
    s.play([Animation::shift(&s.graph, c, Vec2::new(3.0, 1.0), 1.0)]);
    out.push(probe("shift", s, &[0.0, 0.5, 1.0]));

    // scale (about center).
    let mut s = Scene::new();
    let c = s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 1.0))
            .with_style(Style::filled(palette::yellow()).with_stroke(palette::white(), 4.0)),
    );
    s.play([Animation::scale(&s.graph, c, 2.0, 1.0)]);
    out.push(probe("scale", s, &[0.0, 0.5, 1.0]));

    // morph: circle -> triangle.
    let mut s = Scene::new();
    let c = s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 1.5))
            .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 4.0)),
    );
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
        Mobject::new(geometry::rounded_rect(
            Point::new(-4.0, -1.5),
            2.0,
            1.2,
            0.3,
        ))
        .with_style(Style::filled(palette::maroon()).with_stroke(palette::white(), 4.0)),
    );
    s.add(
        Mobject::new(geometry::dashed_line(
            Point::new(-2.2, -1.5),
            Point::new(0.4, -0.6),
            0.18,
            0.1,
        ))
        .with_style(
            Style::default()
                .with_stroke(palette::yellow(), 5.0)
                .no_fill(),
        ),
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
    s.play([Animation::rotate(
        &s.graph,
        tri,
        std::f64::consts::FRAC_PI_2,
        1.0,
    )]);
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
        Mobject::new(geometry::plot(-2.2, 2.2, 80, 1.0, 1.0, |x| 0.35 * x * x)).with_style(
            Style::default()
                .with_stroke(palette::yellow(), 5.0)
                .no_fill(),
        ),
    );
    out.push(probe("axes", s, &[0.0]));

    // plain text (Typst markup, not math).
    let mut s = Scene::new();
    add_text(&mut s.graph, "Hello, Manim", &MathOptions::default()).expect("text compile failed");
    out.push(probe("text", s, &[0.0]));

    // annotations: surrounding rect, underline, brace.
    let mut s = Scene::new();
    let sq = s.add(
        Mobject::new(geometry::square(Point::ORIGIN, 1.6))
            .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 4.0)),
    );
    add_surrounding_rect(
        &mut s.graph,
        sq,
        0.2,
        0.15,
        Style::default()
            .no_fill()
            .with_stroke(palette::yellow(), 4.0),
    );
    add_underline(
        &mut s.graph,
        sq,
        0.15,
        Style::default().with_stroke(palette::gold(), 5.0),
    );
    add_brace(
        &mut s.graph,
        sq,
        DOWN,
        0.15,
        Style::default().with_stroke(palette::white(), 4.0),
    );
    out.push(probe("annotate", s, &[0.0]));

    // number plane
    let mut s = Scene::new();
    add_number_plane(
        &mut s.graph,
        &NumberPlaneOpts {
            x_min: -5.0,
            x_max: 5.0,
            y_min: -3.0,
            y_max: 3.0,
            faded_line_ratio: 2,
            ..NumberPlaneOpts::default()
        },
        Style::default()
            .with_stroke(palette::blue_d(), 2.0)
            .with_opacity(0.7),
        Style::default().with_stroke(palette::white(), 3.0),
    );
    out.push(probe("plane", s, &[0.0]));

    // circumscribe
    let mut s = Scene::new();
    let c = s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 1.2))
            .with_style(Style::filled(palette::green()).with_stroke(palette::white(), 4.0)),
    );
    s.play_circumscribe(c, 1.2, palette::yellow());
    out.push(probe("circumscribe", s, &[0.0, 0.3, 0.6, 0.9, 1.2]));

    // draw border then fill
    let mut s = Scene::new();
    let sq = s.add(
        Mobject::new(geometry::square(Point::ORIGIN, 2.0))
            .with_style(Style::filled(palette::red()).with_stroke(palette::white(), 4.0)),
    );
    s.play_draw_border_then_fill(sq, 1.2);
    out.push(probe("dbtf", s, &[0.0, 0.4, 0.7, 1.2]));

    // wiggle
    let mut s = Scene::new();
    let t = s.add(
        Mobject::new(geometry::triangle(Point::ORIGIN, 1.4))
            .with_style(Style::filled(palette::orange()).with_stroke(palette::white(), 4.0)),
    );
    s.play([Animation::wiggle(&s.graph, t, 1.0)]);
    out.push(probe("wiggle", s, &[0.0, 0.25, 0.5, 0.75, 1.0]));

    // camera zoom toward a circle on the right
    let mut s = Scene::new();
    s.add(
        Mobject::new(geometry::circle(Point::new(-3.0, 0.0), 0.8))
            .with_style(Style::filled(palette::gray()).with_stroke(palette::white(), 3.0)),
    );
    let focus = s.add(
        Mobject::new(geometry::circle(Point::new(2.5, 0.0), 0.8))
            .with_style(Style::filled(palette::teal()).with_stroke(palette::white(), 3.0)),
    );
    s.play_camera_shift(Vec2::new(2.5, 0.0), 0.8);
    s.play_camera_zoom(2.0, 0.8);
    let _ = focus;
    out.push(probe("camera", s, &[0.0, 0.8, 1.6]));

    // star / polygon / annular sector / curved arrow
    let mut s = Scene::new();
    s.add(
        Mobject::new(geometry::star(
            Point::new(-3.5, 0.8),
            5,
            1.1,
            None,
            std::f64::consts::FRAC_PI_2,
        ))
        .with_style(Style::filled(palette::gold()).with_stroke(palette::white(), 3.0)),
    );
    s.add(
        Mobject::new(geometry::regular_polygon(
            Point::new(-0.8, 0.8),
            6,
            1.0,
            0.0,
        ))
        .with_style(Style::filled(palette::teal()).with_stroke(palette::white(), 3.0)),
    );
    s.add(
        Mobject::new(geometry::annular_sector(
            Point::new(2.0, 0.8),
            0.45,
            1.1,
            0.3,
            2.2,
        ))
        .with_style(Style::filled(palette::purple()).with_stroke(palette::white(), 3.0)),
    );
    add_curved_arrow(
        &mut s.graph,
        Point::new(-2.5, -1.8),
        Point::new(2.5, -1.2),
        1.2,
        Style::default().with_stroke(palette::yellow(), 5.0),
    );
    out.push(probe("shapes2", s, &[0.0]));

    // angle + right angle
    let mut s = Scene::new();
    let v = Point::ORIGIN;
    let a = Point::new(2.4, 0.4);
    let b = Point::new(0.6, 2.2);
    s.add(
        Mobject::new(geometry::line(v, a)).with_style(
            Style::default()
                .with_stroke(palette::white(), 4.0)
                .no_fill(),
        ),
    );
    s.add(
        Mobject::new(geometry::line(v, b)).with_style(
            Style::default()
                .with_stroke(palette::white(), 4.0)
                .no_fill(),
        ),
    );
    add_angle(
        &mut s.graph,
        v,
        a,
        b,
        0.7,
        Style::default()
            .with_stroke(palette::yellow(), 5.0)
            .no_fill(),
    );
    add_right_angle(
        &mut s.graph,
        Point::new(-3.0, -1.2),
        Point::new(-1.4, -1.2),
        Point::new(-3.0, 0.4),
        0.35,
        Style::default().with_stroke(palette::teal(), 5.0).no_fill(),
    );
    s.add(
        Mobject::new(geometry::line(
            Point::new(-3.0, -1.2),
            Point::new(-1.2, -1.2),
        ))
        .with_style(
            Style::default()
                .with_stroke(palette::white(), 4.0)
                .no_fill(),
        ),
    );
    s.add(
        Mobject::new(geometry::line(
            Point::new(-3.0, -1.2),
            Point::new(-3.0, 0.6),
        ))
        .with_style(
            Style::default()
                .with_stroke(palette::white(), 4.0)
                .no_fill(),
        ),
    );
    out.push(probe("angle", s, &[0.0]));

    // polar plane
    let mut s = Scene::new();
    add_polar_plane(
        &mut s.graph,
        &PolarPlaneOpts {
            radius: 3.2,
            radius_step: 1.0,
            azimuth_divisions: 12,
            faded_line_ratio: 2,
            ..PolarPlaneOpts::default()
        },
        Style::default()
            .with_stroke(palette::blue_d(), 2.0)
            .with_opacity(0.75),
        Style::default().with_stroke(palette::white(), 3.0),
    );
    out.push(probe("polar", s, &[0.0]));

    // title + decimal
    let mut s = Scene::new();
    add_title(&mut s.graph, "Title", &MathOptions::default()).expect("title");
    let dec = add_decimal(
        &mut s.graph,
        std::f64::consts::PI,
        3,
        &MathOptions::default(),
    )
    .expect("decimal");
    s.graph.set_y(dec, -0.8);
    out.push(probe("title", s, &[0.0]));

    // flash
    let mut s = Scene::new();
    s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 0.25))
            .with_style(Style::filled(palette::yellow()).no_stroke()),
    );
    s.play_flash(Point::ORIGIN, 1.0, palette::yellow());
    out.push(probe("flash", s, &[0.0, 0.35, 0.7, 1.0]));

    // move along path
    let mut s = Scene::new();
    let path = s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 2.0))
            .with_style(Style::default().with_stroke(palette::gray(), 3.0).no_fill()),
    );
    let dot = s.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 0.18))
            .with_style(Style::filled(palette::red()).no_stroke()),
    );
    s.play_move_along_path(dot, path, 1.2);
    out.push(probe("along", s, &[0.0, 0.3, 0.6, 0.9, 1.2]));

    // spin in a star
    let mut s = Scene::new();
    let star = s.add(
        Mobject::new(geometry::star(
            Point::ORIGIN,
            5,
            1.4,
            None,
            std::f64::consts::FRAC_PI_2,
        ))
        .with_style(Style::filled(palette::gold()).with_stroke(palette::white(), 3.0)),
    );
    s.play_spin_in(star, 1.0);
    out.push(probe("spin", s, &[0.0, 0.35, 0.7, 1.0]));

    // area + riemann under a parabola
    let mut s = Scene::new();
    add_axes(
        &mut s.graph,
        &AxesOpts {
            x_min: -3.0,
            x_max: 3.0,
            y_min: -0.5,
            y_max: 2.5,
            unit_size: 1.0,
            include_tip: true,
            ..AxesOpts::default()
        },
        Style::default().with_stroke(palette::gray(), 3.0),
    );
    add_area_under(
        &mut s.graph,
        -2.2,
        2.2,
        80,
        1.0,
        1.0,
        |x| 0.35 * x * x,
        Style::filled(palette::blue())
            .no_stroke()
            .with_opacity(0.35),
    );
    add_riemann_rects(
        &mut s.graph,
        -2.0,
        2.0,
        10,
        1.0,
        1.0,
        |x| 0.35 * x * x,
        RiemannSample::Left,
        palette::teal(),
        palette::green(),
        0.7,
    );
    s.add(
        Mobject::new(geometry::plot(-2.2, 2.2, 80, 1.0, 1.0, |x| 0.35 * x * x)).with_style(
            Style::default()
                .with_stroke(palette::yellow(), 5.0)
                .no_fill(),
        ),
    );
    out.push(probe("riemann", s, &[0.0]));

    // dashed circle
    let mut s = Scene::new();
    let c = s.add(
        Mobject::new(geometry::circle(Point::new(-2.2, 0.0), 1.2)).with_style(
            Style::default()
                .with_stroke(palette::white(), 4.0)
                .no_fill(),
        ),
    );
    let d = add_dashed_copy(&mut s.graph, c, 16, 0.55);
    s.graph.shift(d, Vec2::new(4.4, 0.0));
    s.graph.set_color(d, palette::gold());
    out.push(probe("dashed", s, &[0.0]));

    // complex plane + two points
    let mut s = Scene::new();
    add_complex_plane(
        &mut s.graph,
        &NumberPlaneOpts {
            x_min: -4.0,
            x_max: 4.0,
            y_min: -2.5,
            y_max: 2.5,
            faded_line_ratio: 2,
            ..NumberPlaneOpts::default()
        },
        Style::default()
            .with_stroke(palette::blue_d(), 2.0)
            .with_opacity(0.7),
        Style::default().with_stroke(palette::white(), 3.0),
    );
    s.add(
        Mobject::new(geometry::circle(Point::new(2.0, 1.0), 0.12))
            .with_style(Style::filled(palette::yellow()).no_stroke()),
    );
    s.add(
        Mobject::new(geometry::circle(Point::new(-3.0, -1.5), 0.12))
            .with_style(Style::filled(palette::yellow()).no_stroke()),
    );
    out.push(probe("complex", s, &[0.0]));

    // matrix
    let mut s = Scene::new();
    add_matrix(
        &mut s.graph,
        &[vec![1.0, 2.0], vec![3.0, 4.0]],
        &MathOptions::default(),
    )
    .expect("matrix");
    out.push(probe("matrix", s, &[0.0]));

    // code listing
    let mut s = Scene::new();
    add_code(
        &mut s.graph,
        "fn main() {\n    println!(\"hi\");\n}",
        &MathOptions {
            font_size_pt: 28.0,
            ..MathOptions::default()
        },
    )
    .expect("code");
    out.push(probe("code", s, &[0.0]));

    // number line labels
    let mut s = Scene::new();
    add_number_line(
        &mut s.graph,
        &NumberLineOpts {
            x_min: -3.0,
            x_max: 3.0,
            x_step: 1.0,
            include_tip: true,
            ..NumberLineOpts::default()
        },
        Style::default().with_stroke(palette::white(), 4.0),
    );
    add_number_line_labels(
        &mut s.graph,
        -3.0,
        3.0,
        1.0,
        1.0,
        true,
        &MathOptions {
            font_size_pt: 28.0,
            ..MathOptions::default()
        },
    )
    .expect("labels");
    out.push(probe("labels", s, &[0.0]));

    // fade transform: square → circle
    let mut s = Scene::new();
    let sq = s.add(
        Mobject::new(geometry::square(Point::new(-2.5, 0.0), 1.6))
            .with_style(Style::filled(palette::red()).with_stroke(palette::white(), 4.0)),
    );
    let c = s.add(
        Mobject::new(geometry::circle(Point::new(2.5, 0.0), 0.9))
            .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 4.0)),
    );
    s.play_fade_transform(sq, c, 1.2);
    out.push(probe("fadexform", s, &[0.0, 0.4, 0.8, 1.2]));

    // static arrow field (swirl)
    let mut s = Scene::new();
    add_arrow_field(
        &mut s.graph,
        -3.0,
        3.0,
        -2.0,
        2.0,
        1.0,
        1.0,
        |_x, y| -0.45 * y,
        |x, _y| 0.45 * x,
        0.45,
        Style::default().with_stroke(palette::yellow(), 3.0),
    );
    out.push(probe("field", s, &[0.0]));

    // 2x2 text table
    let mut s = Scene::new();
    add_table(
        &mut s.graph,
        &[vec!["a".into(), "b".into()], vec!["c".into(), "d".into()]],
        &MathOptions {
            font_size_pt: 48.0,
            ..MathOptions::default()
        },
    )
    .expect("table");
    out.push(probe("table", s, &[0.0]));

    // area between y=x^2 and y=0.4
    let mut s = Scene::new();
    add_area_between(
        &mut s.graph,
        -2.0,
        2.0,
        64,
        1.0,
        1.0,
        |x| 0.35 * x * x,
        |_| 0.4,
        Style::filled(palette::blue()).with_opacity(0.55),
    );
    s.add(
        Mobject::new(geometry::plot(-2.2, 2.2, 64, 1.0, 1.0, |x| 0.35 * x * x))
            .with_style(Style::default().with_stroke(palette::yellow(), 4.0)),
    );
    out.push(probe("areax", s, &[0.0]));

    // complex plane with i labels
    let mut s = Scene::new();
    add_complex_plane(
        &mut s.graph,
        &NumberPlaneOpts {
            x_min: -3.0,
            x_max: 3.0,
            y_min: -2.0,
            y_max: 2.0,
            faded_line_ratio: 1,
            ..NumberPlaneOpts::default()
        },
        Style::default()
            .with_stroke(palette::blue_d(), 2.0)
            .with_opacity(0.7),
        Style::default().with_stroke(palette::white(), 3.0),
    );
    add_complex_plane_labels(
        &mut s.graph,
        -3.0,
        3.0,
        1.0,
        -2.0,
        2.0,
        1.0,
        1.0,
        false,
        &MathOptions {
            font_size_pt: 28.0,
            ..MathOptions::default()
        },
    )
    .expect("complex labels");
    out.push(probe("clabels", s, &[0.0]));

    // ChangingDecimal: 1 → 12 via baked atlas
    let mut s = Scene::new();
    let atlas = digit_atlas(&MathOptions::default()).expect("atlas");
    let dec = add_decimal_atlas(&mut s.graph, 1.0, 0, &atlas, &MathOptions::default());
    s.play([Animation::changing_decimal(dec, 1.0, 12.0, 0, atlas, 1.2)]);
    out.push(probe("decimal", s, &[0.0, 0.4, 0.8, 1.2]));

    // arc / sector / annulus
    let mut s = Scene::new();
    s.add(
        Mobject::new(geometry::arc(Point::new(-3.0, 0.0), 1.2, 0.3, 2.2))
            .with_style(Style::default().with_stroke(palette::yellow(), 6.0)),
    );
    s.add(
        Mobject::new(geometry::sector(Point::new(0.0, 0.0), 1.3, 0.4, 1.8)).with_style(
            Style::filled(palette::blue()).with_stroke(palette::white(), 3.0),
        ),
    );
    s.add(
        Mobject::new(geometry::annulus(Point::new(3.0, 0.0), 0.55, 1.2)).with_style(
            Style::filled(palette::red()).with_stroke(palette::white(), 3.0),
        ),
    );
    out.push(probe("rings", s, &[0.0]));

    // TransformMatchingShapes: identical letters rearrange (CE anagram)
    let mut s = Scene::new();
    let src = add_text(&mut s.graph, "CAT", &MathOptions::default()).expect("src");
    s.graph.shift(src, LEFT * 2.4);
    let dst = add_text(&mut s.graph, "ACT", &MathOptions::default()).expect("dst");
    s.graph.shift(dst, RIGHT * 2.4);
    s.play_transform_matching(src, dst, 1.2);
    out.push(probe("anagram", s, &[0.0, 0.4, 0.8, 1.2]));

    // baked graph label sitting to the right of y = 0.35 x^2
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
    let plot = s.add(
        Mobject::new(geometry::plot(-2.2, 2.2, 80, 1.0, 1.0, |x| 0.35 * x * x)).with_style(
            Style::default()
                .with_stroke(palette::yellow(), 5.0)
                .no_fill(),
        ),
    );
    add_graph_label(
        &mut s.graph,
        plot,
        "x^2",
        1.6,
        RIGHT,
        0.2,
        &MathOptions::default(),
    )
    .expect("graph label");
    out.push(probe("glabel", s, &[0.0]));

    // LaggedStart: three Creates, each begins after 0.45 of the previous
    let mut s = Scene::new();
    let circles: Vec<_> = [-2.2, 0.0, 2.2]
        .into_iter()
        .map(|x| {
            s.add(
                Mobject::new(geometry::circle(Point::new(x, 0.0), 0.7))
                    .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 4.0)),
            )
        })
        .collect();
    let anims: Vec<_> = circles
        .iter()
        .map(|&id| Animation::create(&s.graph, id, 0.8))
        .collect();
    s.play_lagged(anims, 0.45);
    out.push(probe("lagged", s, &[0.0, 0.35, 0.7, 1.1, 1.6]));

    // LabeledDot / LabeledLine
    let mut s = Scene::new();
    add_labeled_dot(
        &mut s.graph,
        Point::new(-2.2, 0.4),
        "A",
        manim_core::constants::UP,
        0.18,
        &MathOptions::default(),
    )
    .expect("A");
    add_labeled_dot(
        &mut s.graph,
        Point::new(2.2, 0.4),
        "B",
        manim_core::constants::UP,
        0.18,
        &MathOptions::default(),
    )
    .expect("B");
    add_labeled_line(
        &mut s.graph,
        Point::new(-2.2, 0.4),
        Point::new(2.2, 0.4),
        "c",
        manim_core::constants::DOWN,
        0.2,
        &MathOptions::default(),
    )
    .expect("c");
    out.push(probe("labeled", s, &[0.0]));

    // Elbow + cubic bezier
    let mut s = Scene::new();
    s.add(
        Mobject::new(geometry::elbow(Point::new(-2.2, -0.4), 1.4, 0.0))
            .with_style(Style::default().with_stroke(palette::yellow(), 6.0)),
    );
    s.add(
        Mobject::new(geometry::cubic_bezier(
            Point::new(0.2, -1.2),
            Point::new(1.2, 1.6),
            Point::new(2.4, -1.4),
            Point::new(3.6, 0.8),
        ))
        .with_style(Style::default().with_stroke(palette::blue(), 6.0)),
    );
    out.push(probe("curves", s, &[0.0]));

    // MarkupText + Paragraph
    let mut s = Scene::new();
    let markup = add_markup(
        &mut s.graph,
        r#"<span foreground="blue">Blue</span> is <i>cool</i> and <b>bold</b>"#,
        &MathOptions {
            font_size_pt: 42.0,
            ..MathOptions::default()
        },
    )
    .expect("markup");
    s.graph.move_to(markup, Point::new(0.0, 1.5));
    let para = add_paragraph(
        &mut s.graph,
        "Line one\nLine two is longer\nLine three",
        0.28,
        Some("left"),
        &MathOptions {
            font_size_pt: 36.0,
            ..MathOptions::default()
        },
    )
    .expect("paragraph");
    s.graph.move_to(para, Point::new(0.0, -1.3));
    out.push(probe("markup", s, &[0.0]));

    // Table with inner + outer grid lines
    let mut s = Scene::new();
    add_table_with_lines(
        &mut s.graph,
        &[
            vec!["This".into(), "is a".into()],
            vec!["grid".into(), "table".into()],
        ],
        &MathOptions {
            font_size_pt: 40.0,
            ..MathOptions::default()
        },
        1.1,
        0.7,
        true,
        true,
        Style::default().with_stroke(palette::white(), 2.0),
    )
    .expect("tgrid");
    out.push(probe("tgrid", s, &[0.0]));

    // Bulleted list + math table
    let mut s = Scene::new();
    let list = add_bulleted_list(
        &mut s.graph,
        &["Alpha".into(), "Beta".into(), "Gamma".into()],
        0.4,
        &MathOptions {
            font_size_pt: 40.0,
            ..MathOptions::default()
        },
    )
    .expect("list");
    s.graph.move_to(list, Point::new(-3.4, 0.0));
    let mtable = add_math_table(
        &mut s.graph,
        &[
            vec!["+".into(), "0".into(), "5".into()],
            vec!["2".into(), "2".into(), "7".into()],
        ],
        &MathOptions {
            font_size_pt: 36.0,
            ..MathOptions::default()
        },
        0.7,
        0.45,
        true,
        true,
        Style::default().with_stroke(palette::white(), 2.0),
    )
    .expect("mtable");
    s.graph.move_to(mtable, Point::new(2.4, 0.0));
    out.push(probe("lists", s, &[0.0]));

    // Bar chart
    let mut s = Scene::new();
    add_bar_chart_labeled(
        &mut s.graph,
        &[3.0, 5.0, 2.0, 4.0],
        &["A".into(), "B".into(), "C".into(), "D".into()],
        0.0,
        6.0,
        6.0,
        3.6,
        0.6,
        &[],
        0.8,
        2.0,
        &MathOptions {
            font_size_pt: 28.0,
            ..MathOptions::default()
        },
    )
    .expect("bars");
    out.push(probe("bars", s, &[0.0]));

    // Table with row/col labels and a highlighted cell
    let mut s = Scene::new();
    let table = add_table_labeled(
        &mut s.graph,
        &[vec!["1".into(), "2".into()], vec!["3".into(), "4".into()]],
        &["R1".into(), "R2".into()],
        &["C1".into(), "C2".into()],
        "+",
        &MathOptions {
            font_size_pt: 36.0,
            ..MathOptions::default()
        },
        0.7,
        0.45,
        true,
        true,
        Style::default().with_stroke(palette::white(), 2.0),
    )
    .expect("tlabels");
    add_highlighted_cell(&mut s.graph, table, 4, palette::yellow(), 0.5);
    out.push(probe("tlabels", s, &[0.0]));

    // Triangle, arc-between, curved double arrow, labeled arrow
    let mut s = Scene::new();
    s.add(
        Mobject::new(geometry::triangle(Point::new(-3.4, 0.3), 1.1))
            .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 4.0)),
    );
    s.add(
        Mobject::new(geometry::arc_between_points(
            Point::new(-1.6, -0.8),
            Point::new(0.2, 1.1),
            1.8,
        ))
        .with_style(Style::default().with_stroke(palette::yellow(), 5.0)),
    );
    add_curved_double_arrow(
        &mut s.graph,
        Point::new(0.6, -0.9),
        Point::new(2.4, 1.0),
        1.4,
        Style::default().with_stroke(palette::teal(), 5.0),
    );
    add_labeled_arrow(
        &mut s.graph,
        Point::new(2.8, -1.1),
        Point::new(5.0, 0.6),
        "v",
        manim_core::constants::UP,
        0.18,
        &MathOptions {
            font_size_pt: 36.0,
            ..MathOptions::default()
        },
    )
    .expect("larrow");
    out.push(probe("extras", s, &[0.0]));

    // Write then Unwrite
    let mut s = Scene::new();
    let word = add_text(
        &mut s.graph,
        "Hi",
        &MathOptions {
            font_size_pt: 72.0,
            ..MathOptions::default()
        },
    )
    .expect("hi");
    s.play_write(word, 1.0);
    s.play_unwrite(word, 1.0);
    out.push(probe("unwrite", s, &[0.0, 0.5, 1.0, 1.5, 2.0]));

    // ShowIncreasingSubsets: three dots appear one after another
    let mut s = Scene::new();
    let dots = [
        s.add(
            Mobject::new(geometry::circle(Point::new(-1.6, 0.0), 0.35))
                .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 3.0)),
        ),
        s.add(
            Mobject::new(geometry::circle(Point::new(0.0, 0.0), 0.35))
                .with_style(Style::filled(palette::yellow()).with_stroke(palette::white(), 3.0)),
        ),
        s.add(
            Mobject::new(geometry::circle(Point::new(1.6, 0.0), 0.35))
                .with_style(Style::filled(palette::red()).with_stroke(palette::white(), 3.0)),
        ),
    ];
    let g = s.graph.group_nodes(&dots);
    s.play_show_increasing_subsets(g, 1.5);
    out.push(probe("subsets", s, &[0.0, 0.4, 0.9, 1.5]));

    // CyclicReplace: two squares swap places
    let mut s = Scene::new();
    let left = s.add(
        Mobject::new(geometry::square(Point::new(-1.8, 0.0), 1.1))
            .with_style(Style::filled(palette::teal()).with_stroke(palette::white(), 4.0)),
    );
    let right = s.add(
        Mobject::new(geometry::square(Point::new(1.8, 0.0), 1.1))
            .with_style(Style::filled(palette::purple()).with_stroke(palette::white(), 4.0)),
    );
    s.play_cyclic_replace(&[left, right], 1.0);
    out.push(probe("cyclic", s, &[0.0, 0.5, 1.0]));

    // MathTex parts + set_color_by_tex
    let mut s = Scene::new();
    let eq = add_tex_parts(
        &mut s.graph,
        &["a^2".into(), "+".into(), "b^2".into(), "=".into(), "c^2".into()],
        &MathOptions {
            font_size_pt: 56.0,
            ..MathOptions::default()
        },
        true,
    )
    .expect("texparts");
    set_color_by_tex(&mut s.graph, eq, "a^2", palette::red());
    set_color_by_tex(&mut s.graph, eq, "b^2", palette::blue());
    set_color_by_tex(&mut s.graph, eq, "c^2", palette::yellow());
    out.push(probe("texparts", s, &[0.0]));

    // Network graph with labels
    let mut s = Scene::new();
    add_graph_labeled(
        &mut s.graph,
        &["A".into(), "B".into(), "C".into(), "D".into()],
        &[(0, 1), (1, 2), (2, 3), (3, 0), (0, 2)],
        "circular",
        2.2,
        false,
        0.18,
        Style::filled(palette::blue()).with_stroke(palette::white(), 2.0),
        Style::default().with_stroke(palette::white(), 3.0),
        true,
        &MathOptions {
            font_size_pt: 26.0,
            ..MathOptions::default()
        },
    )
    .expect("graph");
    out.push(probe("graph", s, &[0.0]));

    // Directed tree graph
    let mut s = Scene::new();
    let pos = layout_graph(4, &[(0, 1), (0, 2), (2, 3)], "tree", 2.0);
    add_graph(
        &mut s.graph,
        &pos,
        &[(0, 1), (0, 2), (2, 3)],
        true,
        0.16,
        Style::filled(palette::teal()).with_stroke(palette::white(), 2.0),
        Style::default().with_stroke(palette::yellow(), 3.0),
    );
    out.push(probe("digraph", s, &[0.0]));

    // Tangent + vertical line on y = 0.35 x^2
    let mut s = Scene::new();
    let axes = add_axes(
        &mut s.graph,
        &AxesOpts {
            x_min: -2.5,
            x_max: 2.5,
            y_min: -0.5,
            y_max: 2.5,
            ..AxesOpts::default()
        },
        Style::default().with_stroke(palette::white(), 3.0),
    );
    let plot = s.graph.add_child(
        axes,
        Mobject::new(geometry::plot(-2.2, 2.2, 80, 1.0, 1.0, |x| 0.35 * x * x))
            .with_style(Style::default().with_stroke(palette::yellow(), 5.0)),
    );
    add_tangent_line(
        &mut s.graph,
        plot,
        1.2,
        2.2,
        Style::default().with_stroke(palette::red(), 4.0),
    );
    add_vertical_line_to_graph(
        &mut s.graph,
        plot,
        1.2,
        0.0,
        Style::default().with_stroke(palette::teal(), 3.0),
    );
    out.push(probe("tangent", s, &[0.0]));

    // SVG import: colored shapes parsed once into path groups
    let mut s = Scene::new();
    add_svg(
        &mut s.graph,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 60">
            <circle cx="28" cy="30" r="22" fill="#58C4DD"/>
            <rect x="50" y="10" width="42" height="40" rx="6" fill="#83C167"/>
            <polygon points="18,52 50,6 82,52" fill="#FC6255" fill-opacity="0.9"/>
        </svg>"##,
        3.2,
    )
    .expect("svg");
    out.push(probe("svg", s, &[0.0]));

    // SVG <image> href (data URI) plus a path ring
    let mut s = Scene::new();
    add_svg(
        &mut s.graph,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <image href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAFklEQVR4nGP4/5/hf8SRu/8ZQASIAwBqPQvrM5aq/wAAAABJRU5ErkJggg==" x="10" y="10" width="80" height="80"/>
            <circle cx="50" cy="50" r="46" fill="none" stroke="#FFFFFF" stroke-width="4"/>
        </svg>"##,
        3.4,
    )
    .expect("svgraster");
    out.push(probe("svgraster", s, &[0.0]));

    // Boolean ops: union / intersection / difference of overlapping circles
    let mut s = Scene::new();
    let mk = |s: &mut Scene, c: kurbo::Point| {
        s.add(
            Mobject::new(geometry::circle(c, 0.95))
                .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 2.0)),
        )
    };
    let a = mk(&mut s, Point::new(-3.4, 0.0));
    let b = mk(&mut s, Point::new(-2.4, 0.0));
    add_boolean(
        &mut s.graph,
        a,
        b,
        BooleanOp::Union,
        Style::filled(palette::teal()).with_stroke(palette::white(), 3.0),
    );
    s.graph.get_mut(a).visible = false;
    s.graph.get_mut(b).visible = false;

    let c = mk(&mut s, Point::new(-0.4, 0.0));
    let d = mk(&mut s, Point::new(0.4, 0.0));
    add_boolean(
        &mut s.graph,
        c,
        d,
        BooleanOp::Intersection,
        Style::filled(palette::yellow()).with_stroke(palette::white(), 3.0),
    );
    s.graph.get_mut(c).visible = false;
    s.graph.get_mut(d).visible = false;

    let e = mk(&mut s, Point::new(2.6, 0.0));
    let f = mk(&mut s, Point::new(3.4, 0.0));
    add_boolean(
        &mut s.graph,
        e,
        f,
        BooleanOp::Difference,
        Style::filled(palette::red()).with_stroke(palette::white(), 3.0),
    );
    s.graph.get_mut(e).visible = false;
    s.graph.get_mut(f).visible = false;
    out.push(probe("boolean", s, &[0.0]));

    // ImplicitFunction: unit circle + a hyperbola
    let mut s = Scene::new();
    add_implicit_curve(
        &mut s.graph,
        -2.4,
        2.4,
        -1.6,
        1.6,
        56,
        40,
        |x, y| x * x + y * y - 1.0,
        Style::default().with_stroke(palette::yellow(), 5.0),
    );
    add_implicit_curve(
        &mut s.graph,
        -2.4,
        2.4,
        -1.6,
        1.6,
        56,
        40,
        |x, y| x * x - y * y - 0.6,
        Style::default().with_stroke(palette::teal(), 4.0),
    );
    out.push(probe("implicit", s, &[0.0]));

    // ApplyWave on a horizontal line
    let mut s = Scene::new();
    let wave = s.add(
        Mobject::new(geometry::line(Point::new(-3.2, 0.0), Point::new(3.2, 0.0)))
            .with_style(Style::default().with_stroke(palette::yellow(), 6.0)),
    );
    s.play_apply_wave(wave, 1.0);
    out.push(probe("wave", s, &[0.0, 0.5, 1.0]));

    // FadeTransform stretch: small circle → large square
    let mut s = Scene::new();
    let tiny = s.add(
        Mobject::new(geometry::circle(Point::new(-2.6, 0.0), 0.35))
            .with_style(Style::filled(palette::red()).with_stroke(palette::white(), 4.0)),
    );
    let big = s.add(
        Mobject::new(geometry::square(Point::new(2.4, 0.0), 2.2))
            .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 4.0)),
    );
    s.play_fade_transform(tiny, big, 1.2);
    out.push(probe("stretch", s, &[0.0, 0.6, 1.2]));

    // ImageMobject: checkerboard + a red→blue gradient
    let mut s = Scene::new();
    let check = add_raster(
        &mut s.graph,
        checkerboard(8, 8, [255, 255, 0, 255], [88, 196, 221, 255]),
        3.2,
    );
    s.graph.move_to(check, Point::new(-2.6, 0.0));
    let mut grad = Vec::with_capacity(64 * 16 * 4);
    for _y in 0..16 {
        for x in 0..64 {
            let t = x as f32 / 63.0;
            grad.extend_from_slice(&[
                (252.0 * (1.0 - t) + 88.0 * t) as u8,
                (98.0 * (1.0 - t) + 196.0 * t) as u8,
                (85.0 * (1.0 - t) + 221.0 * t) as u8,
                255,
            ]);
        }
    }
    let strip = add_raster(
        &mut s.graph,
        raster_from_rgba(64, 16, grad).expect("gradient rgba"),
        1.6,
    );
    s.graph.move_to(strip, Point::new(2.4, 0.0));
    out.push(probe("raster", s, &[0.0]));

    // ArcPolygon: quarter-circle sides bulge past the chord square
    let mut s = Scene::new();
    add_arc_polygon(
        &mut s.graph,
        &[
            Point::new(-1.4, -1.4),
            Point::new(1.4, -1.4),
            Point::new(1.4, 1.4),
            Point::new(-1.4, 1.4),
        ],
        std::f64::consts::FRAC_PI_2,
        Style::filled(palette::yellow()).with_stroke(palette::white(), 5.0),
    );
    out.push(probe("arcp", s, &[0.0]));

    // save_state + Restore: faded circle on the right travels home
    let mut s = Scene::new();
    let home = s.add(
        Mobject::new(geometry::circle(Point::new(-2.4, 0.0), 0.7))
            .with_style(Style::filled(palette::yellow()).with_stroke(palette::white(), 4.0)),
    );
    s.graph.save_state(home);
    s.graph.shift(home, Vec2::new(4.8, 0.0));
    s.graph.set_opacity(home, 0.25);
    s.play_restore(home, 1.0);
    out.push(probe("restore", s, &[0.0, 0.5, 1.0]));

    out
}
