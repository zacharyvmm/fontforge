//! Pure-Rust spline operations: simplify, extrema, and direction correction.
//!
//! Implements key spline manipulation algorithms in pure Rust:
//! - `correct_contour_directions`: Ensures outer contours are clockwise,
//!   inner (hole) contours are counter-clockwise.
//! - `simplify_contours`: Removes redundant points (near-colinear, near-duplicate).
//! - `add_extrema_to_contours`: Adds points at curve extrema for cubic Beziers.
//!
//! Operates on point-based contour representations (Vec<Vec<Point>>), which
//! are compatible with the overlap.rs polygon operations and with contours
//! extracted from C SplineSet data.

use std::f64;

// Re-export Point from overlap for convenience.
pub use crate::overlap::Point;

/// A cubic Bezier curve defined by four control points.
/// P0 = start, P1 = first off-curve, P2 = second off-curve, P3 = end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezier {
    pub p0: Point,
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
}

impl CubicBezier {
    /// Evaluate the cubic Bezier at parameter t (0..1).
    pub fn eval(&self, t: f64) -> Point {
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let t2 = t * t;
        Point {
            x: mt2 * mt * self.p0.x
                + 3.0 * mt2 * t * self.p1.x
                + 3.0 * mt * t2 * self.p2.x
                + t2 * t * self.p3.x,
            y: mt2 * mt * self.p0.y
                + 3.0 * mt2 * t * self.p1.y
                + 3.0 * mt * t2 * self.p2.y
                + t2 * t * self.p3.y,
        }
    }

    /// Compute the x-coordinate derivative coefficients.
    /// x'(t) = a*t^2 + b*t + c
    fn x_derivative_coeffs(&self) -> (f64, f64, f64) {
        // x(t) = Ax*t^3 + Bx*t^2 + Cx*t + Dx
        // Ax = -P0x + 3*P1x - 3*P2x + P3x
        // Bx = 3*P0x - 6*P1x + 3*P2x
        // Cx = -3*P0x + 3*P1x
        // x'(t) = 3*Ax*t^2 + 2*Bx*t + Cx
        let ax = -self.p0.x + 3.0 * self.p1.x - 3.0 * self.p2.x + self.p3.x;
        let bx = 3.0 * self.p0.x - 6.0 * self.p1.x + 3.0 * self.p2.x;
        let cx = -3.0 * self.p0.x + 3.0 * self.p1.x;
        (3.0 * ax, 2.0 * bx, cx)
    }

    /// Compute the y-coordinate derivative coefficients.
    fn y_derivative_coeffs(&self) -> (f64, f64, f64) {
        let ay = -self.p0.y + 3.0 * self.p1.y - 3.0 * self.p2.y + self.p3.y;
        let by = 3.0 * self.p0.y - 6.0 * self.p1.y + 3.0 * self.p2.y;
        let cy = -3.0 * self.p0.y + 3.0 * self.p1.y;
        (3.0 * ay, 2.0 * by, cy)
    }

    /// Find t values in (0, 1) where the x-derivative is zero (x-extrema).
    pub fn find_x_extrema(&self) -> Vec<f64> {
        let (a, b, c) = self.x_derivative_coeffs();
        solve_quadratic_in_01(a, b, c)
    }

    /// Find t values in (0, 1) where the y-derivative is zero (y-extrema).
    pub fn find_y_extrema(&self) -> Vec<f64> {
        let (a, b, c) = self.y_derivative_coeffs();
        solve_quadratic_in_01(a, b, c)
    }

    /// Find all t values in (0, 1) where either x or y derivative is zero.
    pub fn find_extrema(&self) -> Vec<f64> {
        let mut ts = self.find_x_extrema();
        ts.extend(self.find_y_extrema());
        // Sort and deduplicate
        ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut result = Vec::new();
        for t in ts {
            if result.is_empty() || (t - result.last().unwrap()).abs() > 1e-10 {
                result.push(t);
            }
        }
        result
    }

    /// Check if this curve is effectively a straight line.
    /// True when control points lie (approximately) on the line between endpoints.
    pub fn is_straight(&self, epsilon: f64) -> bool {
        // Check if P1 and P2 lie on the line from P0 to P3
        let dx = self.p3.x - self.p0.x;
        let dy = self.p3.y - self.p0.y;
        let len_sq = dx * dx + dy * dy;
        if len_sq < 1e-20 {
            return true; // Degenerate
        }
        // Cross product: (P-P0) × (P3-P0)
        let cross1 = (self.p1.x - self.p0.x) * dy - (self.p1.y - self.p0.y) * dx;
        let cross2 = (self.p2.x - self.p0.x) * dy - (self.p2.y - self.p0.y) * dx;
        cross1.abs() / len_sq.sqrt() < epsilon && cross2.abs() / len_sq.sqrt() < epsilon
    }
}

/// Solve quadratic equation a*t^2 + b*t + c = 0 for t in (0, 1).
fn solve_quadratic_in_01(a: f64, b: f64, c: f64) -> Vec<f64> {
    let mut result = Vec::new();
    if a.abs() < 1e-15 {
        // Linear: b*t + c = 0
        if b.abs() > 1e-15 {
            let t = -c / b;
            if t > 1e-10 && t < 1.0 - 1e-10 {
                result.push(t);
            }
        }
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            let sqrt_disc = disc.sqrt();
            let t1 = (-b + sqrt_disc) / (2.0 * a);
            let t2 = (-b - sqrt_disc) / (2.0 * a);
            for t in &[t1, t2] {
                if *t > 1e-10 && *t < 1.0 - 1e-10 {
                    result.push(*t);
                }
            }
            // Sort
            result.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        }
    }
    result
}

/// Check if three points are approximately colinear.
fn points_colinear(a: &Point, b: &Point, c: &Point, epsilon: f64) -> bool {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let bcx = c.x - b.x;
    let bcy = c.y - b.y;
    let cross = abx * bcy - aby * bcx;
    let ab_len = (abx * abx + aby * aby).sqrt();
    let bc_len = (bcx * bcx + bcy * bcy).sqrt();
    if ab_len < epsilon || bc_len < epsilon {
        return true; // Degenerate segment
    }
    (cross / (ab_len * bc_len)).abs() < epsilon
}

/// Check if two points are approximately equal.
fn points_equal(a: &Point, b: &Point, epsilon: f64) -> bool {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy) < epsilon * epsilon
}

/// Remove duplicate and near-duplicate consecutive points from a contour.
fn dedup_contour(points: &[Point], epsilon: f64) -> Vec<Point> {
    if points.is_empty() {
        return vec![];
    }
    let mut result: Vec<Point> = vec![points[0]];
    for i in 1..points.len() {
        if !points_equal(&result[result.len() - 1], &points[i], epsilon) {
            result.push(points[i]);
        }
    }
    // Also check if last equals first (closed contour)
    if result.len() > 1 && points_equal(&result[0], &result[result.len() - 1], epsilon) {
        result.pop();
    }
    result
}

/// Remove colinear interior points from a contour.
/// Keeps the start and end points; removes intermediate points that lie
/// (approximately) on the line between their neighbors.
fn simplify_colinear(points: &[Point], epsilon: f64) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let n = points.len();
    let mut result = vec![points[0]];

    for i in 1..(n - 1) {
        let prev = &result[result.len() - 1];
        let curr = &points[i];
        let next = &points[(i + 1) % n]; // wrap for closed contour

        if !points_colinear(prev, curr, next, epsilon) {
            result.push(*curr);
        }
    }

    // Always keep the last point
    if n > 1 {
        result.push(points[n - 1]);
    }

    // For closed contours: check the wrap-around triplet (n-2, n-1, 0)
    // and the triplet (n-1, 0, 1)
    if result.len() >= 3 {
        // Check if we can remove the first-last connection colinearity
        let m = result.len();
        let first = result[0];
        let second = result[1];
        let last = result[m - 1];
        let _second_last = result[m - 2];

        // If last-first-second are colinear, remove first
        if points_colinear(&last, &first, &second, epsilon) {
            result.remove(0);
        }
        // If second_last-last-first are colinear, remove last
        if result.len() >= 3 {
            let m = result.len();
            if points_colinear(&result[m - 2], &result[m - 1], &result[0], epsilon) {
                result.pop();
            }
        }
    }

    // Remove duplicate closure
    if result.len() > 1 && points_equal(&result[0], &result[result.len() - 1], epsilon) {
        result.pop();
    }

    result
}

/// Simplify a set of contours by removing near-duplicate and colinear points.
///
/// This is a pure-Rust equivalent of the core simplification logic in
/// FontForge's `SplineCharSimplify`. It removes:
/// - Duplicate points (within epsilon)
/// - Colinear interior points (points that lie on a straight line between their neighbors)
///
/// Contours with fewer than 3 points after simplification are dropped.
pub fn simplify_contours(contours: &[Vec<Point>]) -> Vec<Vec<Point>> {
    let epsilon = 1e-3;
    let mut result: Vec<Vec<Point>> = Vec::new();

    for contour in contours {
        if contour.len() < 3 {
            continue;
        }
        let simplified = simplify_colinear(&dedup_contour(contour, epsilon), epsilon);
        if simplified.len() >= 3 {
            result.push(simplified);
        }
    }

    result
}

/// Build cubic Bezier curves from a contour with optional control points.
///
/// Each point i in the contour may have control-point information represented
/// by the two optional Point arrays (next_cps, prev_cps):
/// - next_cps[i] = off-curve control point for the curve leaving point i
/// - prev_cps[i] = off-curve control point for the curve arriving at point i
///
/// If control points are None, line segments are assumed.
pub fn contour_to_beziers(
    contour: &[Point],
    next_cps: &[Option<Point>],
    prev_cps: &[Option<Point>],
) -> Vec<CubicBezier> {
    let n = contour.len();
    let mut beziers = Vec::with_capacity(n);

    for i in 0..n {
        let j = (i + 1) % n;
        let p0 = contour[i];
        let p3 = contour[j];

        let p1 = match next_cps.get(i).and_then(|cp| *cp) {
            Some(cp) => cp,
            None => {
                // No explicit control point: use 1/3 of the way from p0 to p3
                Point {
                    x: p0.x + (p3.x - p0.x) / 3.0,
                    y: p0.y + (p3.y - p0.y) / 3.0,
                }
            }
        };

        let p2 = match prev_cps.get(j).and_then(|cp| *cp) {
            Some(cp) => cp,
            None => {
                // No explicit control point: use 2/3 of the way from p0 to p3
                Point {
                    x: p0.x + 2.0 * (p3.x - p0.x) / 3.0,
                    y: p0.y + 2.0 * (p3.y - p0.y) / 3.0,
                }
            }
        };

        beziers.push(CubicBezier { p0, p1, p2, p3 });
    }

    beziers
}

/// Build cubic Bezier curves from a simple polygon contour (no control points).
/// Each edge of the polygon is treated as a straight line (control points
/// are set at 1/3 and 2/3 along the edge).
pub fn polygon_to_beziers(contour: &[Point]) -> Vec<CubicBezier> {
    let n = contour.len();
    let mut beziers = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let p0 = contour[i];
        let p3 = contour[j];
        let p1 = Point {
            x: p0.x + (p3.x - p0.x) / 3.0,
            y: p0.y + (p3.y - p0.y) / 3.0,
        };
        let p2 = Point {
            x: p0.x + 2.0 * (p3.x - p0.x) / 3.0,
            y: p0.y + 2.0 * (p3.y - p0.y) / 3.0,
        };
        beziers.push(CubicBezier { p0, p1, p2, p3 });
    }
    beziers
}

/// Add extrema points to a single contour represented as cubic Bezier curves.
///
/// For each curve in the contour, finds t-values where x'(t)=0 or y'(t)=0
/// (within (0, 1)), evaluates the curve at those t-values, and inserts the
/// resulting points. This ensures that the contour has explicit on-curve points
/// at all x and y extrema.
///
/// Returns a new contour with extrema points added.
pub fn add_extrema_to_beziers(beziers: &[CubicBezier]) -> Vec<Point> {
    if beziers.is_empty() {
        return vec![];
    }

    let mut result: Vec<Point> = Vec::new();

    for bez in beziers {
        // Add start point
        if result.is_empty() || !points_equal(result.last().unwrap(), &bez.p0, 1e-6) {
            result.push(bez.p0);
        }

        // Find extrema along this curve
        let extrema_ts = bez.find_extrema();

        // Add interior extrema points
        for &t in &extrema_ts {
            let pt = bez.eval(t);
            // Only add if it's not too close to start or end
            if !points_equal(result.last().unwrap(), &pt, 1e-6)
                && !points_equal(&pt, &bez.p3, 1e-6)
            {
                result.push(pt);
            }
        }
    }

    // Add final end point (close the contour)
    if !result.is_empty() {
        let first = result[0];
        let last = beziers.last().unwrap().p3;
        if !points_equal(result.last().unwrap(), &last, 1e-6) {
            result.push(last);
        }
        // Close if needed
        if !points_equal(&last, &first, 1e-6) {
            result.push(first);
        }
    }

    // Remove duplicate closure
    if result.len() > 1 && points_equal(&result[0], &result[result.len() - 1], 1e-6) {
        result.pop();
    }

    result
}

/// Add extrema points to contours represented as simple polygon point lists.
///
/// For a polygonal approximation, each edge is treated as a straight-line
/// Bezier curve. Since straight lines have no interior extrema, this function
/// is essentially a no-op for purely polygonal input. However, it ensures
/// the contour is well-formed.
///
/// For meaningful extrema insertion, use `add_extrema_to_beziers` with
/// proper cubic Bezier data.
pub fn add_extrema_to_contours(contours: &[Vec<Point>]) -> Vec<Vec<Point>> {
    let mut result: Vec<Vec<Point>> = Vec::new();
    for contour in contours {
        if contour.len() < 2 {
            continue;
        }
        let beziers = polygon_to_beziers(contour);
        let with_extrema = add_extrema_to_beziers(&beziers);
        if with_extrema.len() >= 3 {
            result.push(with_extrema);
        }
    }
    result
}

/// Correct contour directions: ensure outer contours are clockwise and
/// inner (hole) contours are counter-clockwise.
///
/// This is the pure-Rust equivalent of `SplineSetsCorrect`.
///
/// Algorithm:
/// 1. For each contour, compute signed area to determine current winding.
/// 2. Build a containment tree: check which contours contain which.
/// 3. Outer contours (even nesting depth) should be clockwise (positive area).
///    Inner contours (odd nesting depth) should be CCW (negative area).
/// 4. Reverse any contour that has the wrong direction.
pub fn correct_contour_directions(contours: &[Vec<Point>]) -> Vec<Vec<Point>> {
    use crate::overlap::{is_clockwise, point_in_polygon};

    if contours.is_empty() {
        return vec![];
    }

    let n = contours.len();
    let mut is_cw: Vec<bool> = Vec::with_capacity(n);
    let mut contains: Vec<Vec<bool>> = vec![vec![false; n]; n];

    for i in 0..n {
        is_cw.push(is_clockwise(&contours[i]));
    }

    // Build containment matrix: contains[i][j] = true if contour i contains contour j
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            // Test if contour j's first point is inside contour i
            if contours[j].len() >= 3 && point_in_polygon(&contours[j][0], &contours[i]) {
                contains[i][j] = true;
            }
        }
    }

    // Compute nesting depth for each contour.
    // Nesting depth = number of contours that contain this contour.
    let mut depth: Vec<usize> = vec![0; n];
    for j in 0..n {
        for i in 0..n {
            if contains[i][j] {
                depth[j] += 1;
            }
        }
    }

    // Correct directions:
    // Even depth -> should be CW (outer contour)
    // Odd depth  -> should be CCW (hole)
    let mut result: Vec<Vec<Point>> = Vec::with_capacity(n);
    for i in 0..n {
        let should_be_cw = (depth[i] % 2) == 0;
        let mut contour = contours[i].clone();
        if should_be_cw != is_cw[i] {
            contour.reverse();
        }
        result.push(contour);
    }

    result
}

/// Compute the bounding box of a point.
pub fn point_bbox(p: &Point) -> (f64, f64, f64, f64) {
    (p.x, p.y, p.x, p.y)
}

/// Compute the axis-aligned bounding box of multiple contours.
pub fn contours_bbox(contours: &[Vec<Point>]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for contour in contours {
        for pt in contour {
            if pt.x < min_x {
                min_x = pt.x;
            }
            if pt.y < min_y {
                min_y = pt.y;
            }
            if pt.x > max_x {
                max_x = pt.x;
            }
            if pt.y > max_y {
                max_y = pt.y;
            }
        }
    }

    (min_x, min_y, max_x, max_y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlap::Point;

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn test_cubic_bezier_eval() {
        // Bezier: P0=(0,0), P1=(10,0), P2=(10,10), P3=(0,10)
        // At t=0.5: x = 7.5, y = 5.0
        let b = CubicBezier {
            p0: pt(0.0, 0.0),
            p1: pt(10.0, 0.0),
            p2: pt(10.0, 10.0),
            p3: pt(0.0, 10.0),
        };
        let mid = b.eval(0.5);
        assert!((mid.x - 7.5).abs() < 1e-6);
        assert!((mid.y - 5.0).abs() < 1e-6);

        // At t=0: should be P0
        let start = b.eval(0.0);
        assert!((start.x - 0.0).abs() < 1e-6);
        assert!((start.y - 0.0).abs() < 1e-6);

        // At t=1: should be P3
        let end = b.eval(1.0);
        assert!((end.x - 0.0).abs() < 1e-6);
        assert!((end.y - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_find_x_extrema_straight_line() {
        let b = CubicBezier {
            p0: pt(0.0, 0.0),
            p1: pt(33.0, 0.0),
            p2: pt(66.0, 0.0),
            p3: pt(100.0, 0.0),
        };
        // Straight horizontal line: x'(t) is constant (non-zero), no interior extrema
        let extrema = b.find_x_extrema();
        assert!(extrema.is_empty(), "Straight line should have no x-extrema");
    }

    #[test]
    fn test_find_x_extrema_curve() {
        // A curve that goes right then left: P0=(0,0), P1=(100,0), P2=(0,100), P3=(100,100)
        let b = CubicBezier {
            p0: pt(0.0, 0.0),
            p1: pt(100.0, 0.0),
            p2: pt(0.0, 100.0),
            p3: pt(100.0, 100.0),
        };
        let extrema = b.find_x_extrema();
        assert!(!extrema.is_empty(), "Curve should have x-extrema");
        for &t in &extrema {
            assert!(t > 0.0 && t < 1.0, "Extrema should be interior");
        }
    }

    #[test]
    fn test_is_straight() {
        let straight = CubicBezier {
            p0: pt(0.0, 0.0),
            p1: pt(33.0, 0.0),
            p2: pt(66.0, 0.0),
            p3: pt(100.0, 0.0),
        };
        assert!(straight.is_straight(1e-3));

        let curved = CubicBezier {
            p0: pt(0.0, 0.0),
            p1: pt(50.0, 50.0),
            p2: pt(50.0, 0.0),
            p3: pt(100.0, 0.0),
        };
        assert!(!curved.is_straight(1e-3));
    }

    #[test]
    fn test_solve_quadratic() {
        // t^2 - 2t + 1 = (t-1)^2 = 0 => t=1 (outside (0,1))
        let result = solve_quadratic_in_01(1.0, -2.0, 1.0);
        assert!(result.is_empty());

        // t^2 - t = t(t-1) = 0 => t=0,1 (on boundary, not interior)
        let result = solve_quadratic_in_01(1.0, -1.0, 0.0);
        assert!(result.is_empty());

        // t^2 - 0.5 = 0 => t = sqrt(0.5) ≈ 0.707
        let result = solve_quadratic_in_01(1.0, 0.0, -0.5);
        assert_eq!(result.len(), 1);
        assert!((result[0] - 0.70710678).abs() < 1e-6);
    }

    #[test]
    fn test_simplify_colinear() {
        // Square with an extra colinear point on one edge
        let points = vec![
            pt(0.0, 0.0),
            pt(5.0, 0.0),  // colinear extra
            pt(10.0, 0.0),
            pt(10.0, 10.0),
            pt(0.0, 10.0),
        ];
        let simplified = simplify_colinear(&points, 1e-3);
        assert_eq!(simplified.len(), 4, "Should reduce to 4 points (square corners)");
    }

    #[test]
    fn test_simplify_contours() {
        // Two overlapping rectangles with extra points
        let r1 = vec![
            pt(0.0, 0.0),
            pt(5.0, 0.0),
            pt(10.0, 0.0),
            pt(10.0, 10.0),
            pt(0.0, 10.0),
        ];
        let r2 = vec![
            pt(5.0, 5.0),
            pt(10.0, 5.0),
            pt(15.0, 5.0),
            pt(15.0, 15.0),
            pt(5.0, 15.0),
        ];
        let simplified = simplify_contours(&[r1, r2]);
        assert_eq!(simplified.len(), 2);
        // Each should be simplified to 4 corners
        assert_eq!(simplified[0].len(), 4);
        assert_eq!(simplified[1].len(), 4);
    }

    #[test]
    fn test_correct_direction_basic() {
        use crate::overlap::is_clockwise;

        // Clockwise square
        let cw = vec![
            pt(0.0, 0.0),
            pt(10.0, 0.0),
            pt(10.0, 10.0),
            pt(0.0, 10.0),
        ];
        assert!(is_clockwise(&cw));

        // Counter-clockwise square (same as above reversed)
        let ccw = vec![
            pt(0.0, 0.0),
            pt(0.0, 10.0),
            pt(10.0, 10.0),
            pt(10.0, 0.0),
        ];
        assert!(!is_clockwise(&ccw));

        // Correct a CCW outer contour -> should become CW
        let corrected = correct_contour_directions(&[ccw.clone()]);
        assert_eq!(corrected.len(), 1);
        assert!(is_clockwise(&corrected[0]));

        // CW outer contour should stay CW
        let corrected = correct_contour_directions(&[cw.clone()]);
        assert_eq!(corrected.len(), 1);
        assert!(is_clockwise(&corrected[0]));
    }

    #[test]
    fn test_correct_direction_hole() {
        use crate::overlap::is_clockwise;

        // Outer contour (large CW square)
        let outer = vec![
            pt(0.0, 0.0),
            pt(100.0, 0.0),
            pt(100.0, 100.0),
            pt(0.0, 100.0),
        ];
        // Inner contour (small CW square that should become CCW)
        let inner = vec![
            pt(25.0, 25.0),
            pt(75.0, 25.0),
            pt(75.0, 75.0),
            pt(25.0, 75.0),
        ];

        let corrected = correct_contour_directions(&[outer, inner]);
        assert_eq!(corrected.len(), 2);
        assert!(is_clockwise(&corrected[0]), "Outer should be CW");
        assert!(!is_clockwise(&corrected[1]), "Inner (hole) should be CCW");
    }

    #[test]
    fn test_add_extrema_to_beziers() {
        // A curve that definitely has extrema
        let b = vec![CubicBezier {
            p0: pt(0.0, 0.0),
            p1: pt(100.0, 0.0),
            p2: pt(0.0, 100.0),
            p3: pt(100.0, 100.0),
        }];
        let points = add_extrema_to_beziers(&b);
        // At minimum, we get start and end points
        assert!(points.len() >= 3, "Should have at least start, extrema, end");
    }

    #[test]
    fn test_contours_bbox() {
        let contours = vec![
            vec![pt(0.0, 0.0), pt(10.0, 0.0), pt(10.0, 10.0), pt(0.0, 10.0)],
            vec![pt(-5.0, -5.0), pt(5.0, -5.0), pt(5.0, 5.0), pt(-5.0, 5.0)],
        ];
        let (min_x, min_y, max_x, max_y) = contours_bbox(&contours);
        assert_eq!(min_x, -5.0);
        assert_eq!(min_y, -5.0);
        assert_eq!(max_x, 10.0);
        assert_eq!(max_y, 10.0);
    }
}
