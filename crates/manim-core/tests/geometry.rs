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
