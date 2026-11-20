//! The shape sphere's projection, a display remap of the spherical map's render (`kernel::chart::shape_sphere`;
//! chart_reference §3.3, "Projection" and "The equal-area projection is Hammer's"; TASK-M2-28).
//! - `shape_projection_default`: equirectangular by default, its display point the chart point both ways
//!   (REQ-RENDER-026).
//! - `shape_projection_hammer_landmarks`: Hammer's centre is `θ = π` on the equator, its top and bottom the poles
//!   `L⁺` and `L⁻`, its ends `θ = 0` and `2π`; the ellipse's edge `q = 1` is shown, past it nothing (REQ-CHART-047).
//! - `shape_projection_hammer_round_trip`: the inverse and the forward map undo each other, to `10⁻¹²`, over the
//!   chart and over the ellipse (REQ-CHART-047).
//! - `shape_projection_hammer_equal_area`: the display's area element covers the solid angle `16 dx dy` everywhere
//!   inside the ellipse, by finite differences of the shape-sphere point it shows, to `10⁻⁵`, where the
//!   equirectangular's varies with `sin φ` (REQ-RENDER-026, REQ-CHART-047).

use kernel::chart::shape_sphere::{hammer_forward, hammer_inverse, spherical, Projection};
use validation::negative_control;

const PI: f64 = std::f64::consts::PI;

/// A display remap from the display point to the chart point it shows.
type Remap = fn(f64, f64) -> Option<[f64; 2]>;

/// `a` and `b` within `tol`, coordinate by coordinate.
fn near(a: [f64; 2], b: [f64; 2], tol: f64) -> bool {
    (a[0] - b[0]).abs() <= tol && (a[1] - b[1]).abs() <= tol
}

#[test]
fn shape_projection_default() {
    assert_eq!(Projection::default(), Projection::Equirectangular);
    for (x, y) in [(0.0, 0.0), (0.25, 0.75), (1.0, 1.0)] {
        assert_eq!(
            Projection::Equirectangular.chart_point(x, y),
            Some([x, y]),
            "the default is not the identity"
        );
        assert_eq!(Projection::Equirectangular.display_point(x, y), [x, y]);
    }
}

negative_control!(
    shape_projection_default,
    "Hammer's taken for the default moves the display point, so it must fail",
    expected = "the default is not the identity",
    {
        for (x, y) in [(0.0, 0.0), (0.25, 0.75), (1.0, 1.0)] {
            assert_eq!(
                Projection::EqualArea.chart_point(x, y),
                Some([x, y]),
                "the default is not the identity"
            );
        }
    }
);

/// Hammer's landmarks through `inverse`: the centre, the poles at the top and bottom, the ends of the equator at the
/// ellipse's sides, each on the edge `q = 1` and shown; past the edge and at the square's corners, nothing.
fn check_landmarks(inverse: Remap) {
    let shown = |x: f64, y: f64| {
        inverse(x, y).unwrap_or_else(|| panic!("({x}, {y}) inside the ellipse shows nothing"))
    };
    assert!(
        near(shown(0.5, 0.5), [0.5, 0.5], 1e-15),
        "the centre is not θ = π on the equator"
    );
    for (y, t) in [(1.0, 1.0), (0.0, 0.0)] {
        let p = shown(0.5, y);
        assert_eq!(
            p[1], t,
            "the display's edge y = {y} is not the pole t = {t}"
        );
    }
    assert!(
        near(shown(1.0, 0.5), [1.0, 0.5], 1e-15),
        "the right end is not θ = 2π"
    );
    assert!(
        near(shown(0.0, 0.5), [0.0, 0.5], 1e-15),
        "the left end is not θ = 0"
    );
    for (x, y) in [
        (1.0 + 1e-9, 0.5),
        (0.5, 1.0 + 1e-9),
        (0.0, 0.0),
        (1.0, 1.0),
        (0.9, 0.9),
    ] {
        assert_eq!(
            inverse(x, y),
            None,
            "({x}, {y}) outside the ellipse shows a chart point"
        );
    }
}

#[test]
fn shape_projection_hammer_landmarks() {
    check_landmarks(hammer_inverse);
    assert_eq!(
        Projection::EqualArea.chart_point(0.5, 1.0),
        hammer_inverse(0.5, 1.0)
    );
}

negative_control!(
    shape_projection_hammer_landmarks,
    "an ellipse open at its edge, q < 1, drops the poles and the equator's ends, so it must fail",
    expected = "inside the ellipse shows nothing",
    {
        check_landmarks(|x, y| {
            let q = (2.0 * x - 1.0).powi(2) + (2.0 * y - 1.0).powi(2);
            if q >= 1.0 {
                None
            } else {
                hammer_inverse(x, y)
            }
        });
    }
);

/// `inverse` undoes `hammer_forward` over a grid of chart points, and `hammer_forward` undoes `inverse` over a grid of
/// display points inside the ellipse, to `10⁻¹²`.
fn check_round_trip(inverse: Remap) {
    for i in 1..40 {
        for j in 1..40 {
            let (s, t) = (f64::from(i) / 40.0, f64::from(j) / 40.0);
            let [x, y] = hammer_forward(s, t);
            let back = inverse(x, y).expect("a chart point's display point lies in the ellipse");
            assert!(
                near(back, [s, t], 1e-12),
                "({s}, {t}) comes back as {back:?}"
            );
            assert_eq!(Projection::EqualArea.display_point(s, t), [x, y]);
        }
    }
    for i in 0..=40 {
        for j in 0..=40 {
            let (x, y) = (f64::from(i) / 40.0, f64::from(j) / 40.0);
            if let Some([s, t]) = inverse(x, y) {
                if (2.0 * y - 1.0).abs() < 1.0 {
                    assert!(
                        near(hammer_forward(s, t), [x, y], 1e-12),
                        "({x}, {y}) comes back off"
                    );
                }
            }
        }
    }
}

#[test]
fn shape_projection_hammer_round_trip() {
    check_round_trip(hammer_inverse);
}

negative_control!(
    shape_projection_hammer_round_trip,
    "the longitude without its factor 2, Aitoff's half angle read as the whole, must fail",
    expected = "comes back as",
    {
        check_round_trip(|x, y| hammer_inverse(x, y).map(|[s, t]| [(s - 0.5) / 2.0 + 0.5, t]));
    }
);

/// The shape-sphere point the display point `(x, y)` shows under `remap`, through the spherical map.
fn point(remap: Remap, x: f64, y: f64) -> [f64; 3] {
    let [s, t] = remap(x, y).expect("the display point shows a chart point");
    spherical(s, t, PI)
}

/// The solid angle per unit display area at `(x, y)` under `remap`: `‖∂n/∂x × ∂n/∂y‖` by central differences.
fn density(remap: Remap, x: f64, y: f64) -> f64 {
    let h = 1e-6;
    let diff = |a: [f64; 3], b: [f64; 3]| [0, 1, 2].map(|k| (a[k] - b[k]) / (2.0 * h));
    let dx = diff(point(remap, x + h, y), point(remap, x - h, y));
    let dy = diff(point(remap, x, y + h), point(remap, x, y - h));
    let c = [
        dx[1] * dy[2] - dx[2] * dy[1],
        dx[2] * dy[0] - dx[0] * dy[2],
        dx[0] * dy[1] - dx[1] * dy[0],
    ];
    (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt()
}

/// `remap` is equal-area: `16` everywhere inside the ellipse, to `10⁻⁵`.
fn check_equal_area(remap: Remap) {
    for (x, y) in [
        (0.5, 0.5),
        (0.3, 0.6),
        (0.8, 0.2),
        (0.15, 0.45),
        (0.55, 0.92),
        (0.62, 0.07),
    ] {
        let a = density(remap, x, y);
        assert!(
            (a - 16.0).abs() < 1e-5,
            "the area element at ({x}, {y}) covers {a} sr, not 16"
        );
    }
}

#[test]
fn shape_projection_hammer_equal_area() {
    check_equal_area(hammer_inverse);
    // The equirectangular's element is 2π²·sin φ: 2π² on the equator, a tenth of it at sin φ = 0.1.
    let equator = density(|x, y| Some([x, y]), 0.3, 0.5);
    assert!((equator - 2.0 * PI * PI).abs() < 1e-5);
}

negative_control!(
    shape_projection_hammer_equal_area,
    "the equirectangular display, whose area element shrinks toward the poles, is not equal-area, so it must fail",
    expected = "not 16",
    {
        check_equal_area(|x, y| {
            let q = (2.0 * x - 1.0).powi(2) + (2.0 * y - 1.0).powi(2);
            (q <= 1.0).then_some([x, y])
        });
    }
);
