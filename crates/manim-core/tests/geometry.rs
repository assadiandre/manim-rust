use std::f64::consts::PI;

use kurbo::Point;
use manim_core::geometry;

#[test]
fn circle_length_is_circumference() {
    let c = geometry::circle(Point::ORIGIN, 2.0);
    let len = geometry::path_length(&c);
    assert!((len - 4.0 * PI).abs() < 1e-3, "got {len}");
}

#[test]
fn resample_gives_even_spacing() {
    let sq = geometry::square(Point::ORIGIN, 2.0);
    let (pts, closed) = geometry::resample(&sq, 64);
    assert!(closed);
    assert_eq!(pts.len(), 64);
    let perim = 8.0;
    let expected = perim / 64.0;
    for w in pts.windows(2) {
        let d = w[0].distance(w[1]);
        assert!((d - expected).abs() < 0.05, "uneven spacing {d}");
    }
}

#[test]
fn trim_half_is_half_length() {
    let c = geometry::circle(Point::ORIGIN, 1.0);
    let half = geometry::trim(&c, 0.0, 0.5);
    let len = geometry::path_length(&half);
    let full = geometry::path_length(&c);
    assert!((len - full / 2.0).abs() / full < 0.02, "len={len} full={full}");
}

#[test]
fn trim_zero_is_empty() {
    let c = geometry::circle(Point::ORIGIN, 1.0);
    assert!(geometry::trim(&c, 0.0, 0.0).elements().is_empty());
}

#[test]
fn ellipse_is_stretched_circle() {
    let e = geometry::ellipse(Point::ORIGIN, 2.0, 1.0);
    let bb = geometry::bounding_box(&e);
    assert!((bb.width() - 4.0).abs() < 0.05, "w={}", bb.width());
    assert!((bb.height() - 2.0).abs() < 0.05, "h={}", bb.height());
}

#[test]
fn arc_quarter_is_quarter_circle() {
    let a = geometry::arc(Point::ORIGIN, 1.0, 0.0, PI / 2.0);
    let len = geometry::path_length(&a);
    assert!((len - PI / 2.0).abs() < 0.02, "len={len}");
}

#[test]
fn dashed_line_is_shorter_than_solid() {
    let a = Point::new(-2.0, 0.0);
    let b = Point::new(2.0, 0.0);
    let solid = geometry::path_length(&geometry::line(a, b));
    let dashed = geometry::path_length(&geometry::dashed_line(a, b, 0.2, 0.2));
    assert!(dashed < solid * 0.7 && dashed > solid * 0.3, "dashed={dashed} solid={solid}");
}

#[test]
fn arrow_shaft_stops_before_tip() {
    let start = Point::new(-2.0, 0.0);
    let end = Point::new(2.0, 0.0);
    let shaft = geometry::arrow_shaft(start, end, 0.4);
    let bb = geometry::bounding_box(&shaft);
    assert!(bb.x1 < 2.0 - 0.3, "shaft should end before the tip, x1={}", bb.x1);
}

#[test]
fn plot_samples_a_parabola() {
    let p = geometry::plot(-1.0, 1.0, 9, 1.0, 1.0, |x| x * x);
    let (pts, closed) = geometry::flatten_points(&p);
    assert!(!closed);
    assert!(pts.len() >= 9);
    assert!((pts[0].y - 1.0).abs() < 1e-9);
    assert!(pts[4].y.abs() < 1e-9); // x=0
}

#[test]
fn star_has_ten_vertices() {
    let s = geometry::star(Point::ORIGIN, 5, 1.0, None, std::f64::consts::FRAC_PI_2);
    let (pts, closed) = geometry::flatten_points(&s);
    assert!(closed);
    assert_eq!(pts.len(), 10);
    let tip = pts
        .iter()
        .max_by(|a, b| a.y.partial_cmp(&b.y).unwrap())
        .unwrap();
    assert!(tip.y > 0.9 && tip.x.abs() < 0.05, "{tip:?}");
}

#[test]
fn arc_between_points_quarter_circle() {
    let a = geometry::arc_between_points(Point::new(1.0, 0.0), Point::new(0.0, 1.0), PI / 2.0);
    let len = geometry::path_length(&a);
    assert!((len - PI / 2.0).abs() < 0.05, "len={len}");
    let start = geometry::point_along(&a, 0.0);
    let end = geometry::point_along(&a, 1.0);
    assert!((start.x - 1.0).abs() < 0.02 && start.y.abs() < 0.02, "{start:?}");
    assert!(end.x.abs() < 0.02 && (end.y - 1.0).abs() < 0.02, "{end:?}");
}

#[test]
fn lerp_paths_endpoints_match() {
    let c = geometry::circle(Point::ORIGIN, 1.0);
    let s = geometry::square(Point::ORIGIN, 2.0);
    let at0 = geometry::lerp_paths(&c, &s, 256, 0.0);
    let at1 = geometry::lerp_paths(&c, &s, 256, 1.0);
    let lc = geometry::path_length(&c);
    let ls = geometry::path_length(&s);
    assert!((geometry::path_length(&at0) - lc).abs() / lc < 0.05);
    assert!((geometry::path_length(&at1) - ls).abs() / ls < 0.05);
}

#[test]
fn dashed_path_is_shorter_with_subpaths() {
    let solid = geometry::circle(Point::ORIGIN, 1.0);
    let dashed = geometry::dashed_path(&solid, 8, 0.5);
    let sl = geometry::path_length(&solid);
    let dl = geometry::path_length(&dashed);
    assert!(dl < sl * 0.7 && dl > sl * 0.3, "dashed={dl} solid={sl}");
    let moves = dashed
        .elements()
        .iter()
        .filter(|e| matches!(e, manim_core::kurbo::PathEl::MoveTo(_)))
        .count();
    assert!(moves >= 2, "expected multiple subpaths, got {moves}");

    let almost_solid = geometry::dashed_path(&solid, 8, 1.0);
    assert!(!almost_solid.elements().is_empty());
    let al = geometry::path_length(&almost_solid);
    assert!(al > sl * 0.8 && al < sl, "ratio=1.0-ish len={al} solid={sl}");
}

#[test]
fn area_under_parabola_closes_with_height() {
    let p = geometry::area_under(-1.0, 1.0, 32, 1.0, 1.0, |x| x * x);
    let (_, closed) = geometry::flatten_points(&p);
    assert!(closed);
    let bb = geometry::bounding_box(&p);
    assert!(bb.height() > 0.5, "h={}", bb.height());
}
