//! Pure-Rust spline overlap removal.
//!
//! Implements a polygon-based boolean union algorithm for removing
//! overlapping regions from closed contours. Bezier curves are flattened
//! to line segments at sufficient resolution before processing.
//!
//! The algorithm:
//! 1. Find all intersections between edges across all contours
//! 2. Split edges at intersection points
//! 3. Discard edge fragments whose midpoint lies inside any *other* contour
//!    (using the even-odd winding rule)
//! 4. Trace the remaining edge fragments into closed output contours

use std::f64;

/// A 2D point in the coordinate plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    fn midpoint(&self, other: &Point) -> Point {
        Point {
            x: (self.x + other.x) / 2.0,
            y: (self.y + other.y) / 2.0,
        }
    }

    fn distance_sq(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    fn roughly_equals(&self, other: &Point, epsilon: f64) -> bool {
        self.distance_sq(other) < epsilon * epsilon
    }
}

/// A directed edge fragment: from `start` to `end` belonging to `orig_contour_idx`.
#[derive(Debug, Clone)]
struct EdgeFragment {
    start: Point,
    end: Point,
    orig_contour_idx: usize,
}

/// Check if a point is strictly inside a polygon (not on its boundary).
/// Uses the even-odd rule but also checks that the point is not within
/// epsilon of any edge.
pub fn point_strictly_inside_polygon(point: &Point, polygon: &[Point], epsilon: f64) -> bool {
    if !point_in_polygon(point, polygon) {
        return false;
    }
    // Check if the point is on any edge
    let n = polygon.len();
    for i in 0..n {
        let j = (i + 1) % n;
        if point_on_segment(point, &polygon[i], &polygon[j], epsilon) {
            return false; // On boundary, not strictly inside
        }
    }
    true
}

/// Check if a point lies on a line segment (within epsilon distance).
fn point_on_segment(point: &Point, a: &Point, b: &Point, epsilon: f64) -> bool {
    let ab_x = b.x - a.x;
    let ab_y = b.y - a.y;
    let ap_x = point.x - a.x;
    let ap_y = point.y - a.y;

    let len_sq = ab_x * ab_x + ab_y * ab_y;
    if len_sq < 1e-20 {
        // Degenerate segment: check if point equals a
        return point.roughly_equals(a, epsilon);
    }

    // Project point onto the line. t = dot(ap, ab) / len_sq
    let t = (ap_x * ab_x + ap_y * ab_y) / len_sq;

    if t < -epsilon || t > 1.0 + epsilon {
        return false; // Not within the segment extent
    }

    // Compute distance from point to the line
    // Cross product of AB and AP
    let cross = ab_x * ap_y - ab_y * ap_x;
    let dist = cross.abs() / len_sq.sqrt();

    dist < epsilon
}

/// Check if a point is inside a closed polygon using the even-odd (ray casting) rule.
/// The polygon is assumed closed: last point connects to first.
pub fn point_in_polygon(point: &Point, polygon: &[Point]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let pi = &polygon[i];
        let pj = &polygon[j];
        // Check if the ray from point to +x crosses the edge
        if (pi.y > point.y) != (pj.y > point.y) {
            let x_intersect = pj.x + (pi.x - pj.x) * (point.y - pj.y) / (pi.y - pj.y);
            if point.x < x_intersect {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Compute the signed area (×2) of a polygon. Positive = clockwise (outer contour),
/// negative = counter-clockwise (hole).
pub fn signed_area_2x(polygon: &[Point]) -> f64 {
    let n = polygon.len();
    let mut area = 0.0;
    let mut j = n - 1;
    for i in 0..n {
        area += (polygon[j].x + polygon[i].x) * (polygon[i].y - polygon[j].y);
        j = i;
    }
    area
}

/// Determine if a polygon is clockwise (positive signed area).
pub fn is_clockwise(polygon: &[Point]) -> bool {
    signed_area_2x(polygon) > 0.0
}

/// Result of testing two segments for intersection.
#[derive(Debug)]
enum SegmentRelation {
    /// Proper interior intersection (t1, t2 both in (0,1))
    ProperInterior(f64, f64),
    /// Intersection at a vertex of one or both segments (clamped to [0,1])
    VertexInterior(f64, f64),
    /// Collinear overlapping segments
    CollinearOverlap {
        /// Parameter range on first segment of the overlap
        t1_start: f64,
        t1_end: f64,
        /// Parameter range on second segment of the overlap
        t2_start: f64,
        t2_end: f64,
    },
    /// No intersection
    None,
}

/// Test two line segments for intersection, including endpoint and collinear cases.
/// Returns the intersection relationship.
fn segment_relation(p1: &Point, p2: &Point, q1: &Point, q2: &Point, epsilon: f64) -> SegmentRelation {
    let d1x = p2.x - p1.x;
    let d1y = p2.y - p1.y;
    let d2x = q2.x - q1.x;
    let d2y = q2.y - q1.y;

    let cross = d1x * d2y - d1y * d2x;

    if cross.abs() < 1e-15 {
        // Parallel or collinear
        // Check if they are collinear by testing if q1 lies on the line of (p1,p2)
        let q1_cross = (q1.x - p1.x) * d1y - (q1.y - p1.y) * d1x;
        if q1_cross.abs() > epsilon {
            return SegmentRelation::None;
        }

        // Collinear — find the overlap range
        // Project all points onto the direction vector
        let len_sq = d1x * d1x + d1y * d1y;
        if len_sq < 1e-20 {
            return SegmentRelation::None; // Degenerate segment
        }

        let project = |pt: &Point| -> f64 {
            ((pt.x - p1.x) * d1x + (pt.y - p1.y) * d1y) / len_sq
        };

        let tp1 = 0.0;
        let tp2 = 1.0;
        let tq1 = project(q1);
        let tq2 = project(q2);

        // Normalize q params: ensure tq1 <= tq2 when sorted
        let (tq_min, tq_max) = if tq1 <= tq2 { (tq1, tq2) } else { (tq2, tq1) };

        // Find overlap range
        let overlap_start = f64::max(tp1, tq_min);
        let overlap_end = f64::min(tp2, tq_max);

        if overlap_start < overlap_end - epsilon {
            // Map back to segment-local parameters
            // Overlap on p segment: [overlap_start, overlap_end]
            // Overlap on q segment: need to map back
            let t2_start = if tq1 <= tq2 {
                (overlap_start - tq1) / (tq_max - tq_min)
            } else {
                (tq_max - overlap_start) / (tq_max - tq_min)
            };
            let t2_end = if tq1 <= tq2 {
                (overlap_end - tq1) / (tq_max - tq_min)
            } else {
                (tq_max - overlap_end) / (tq_max - tq_min)
            };

            return SegmentRelation::CollinearOverlap {
                t1_start: overlap_start,
                t1_end: overlap_end,
                t2_start: t2_start.min(t2_end),
                t2_end: t2_start.max(t2_end),
            };
        }

        return SegmentRelation::None;
    }

    let dqx = p1.x - q1.x;
    let dqy = p1.y - q1.y;

    let t1 = (d2x * dqy - d2y * dqx) / cross;
    let t2 = (d1x * dqy - d1y * dqx) / cross;

    // Determine if intersection is proper or at a vertex
    let t1_in = t1 >= -epsilon && t1 <= 1.0 + epsilon;
    let t2_in = t2 >= -epsilon && t2 <= 1.0 + epsilon;

    if !t1_in || !t2_in {
        return SegmentRelation::None;
    }

    let t1_proper = t1 > epsilon && t1 < 1.0 - epsilon;
    let t2_proper = t2 > epsilon && t2 < 1.0 - epsilon;

    let t1 = t1.clamp(0.0, 1.0);
    let t2 = t2.clamp(0.0, 1.0);

    if t1_proper && t2_proper {
        SegmentRelation::ProperInterior(t1, t2)
    } else {
        SegmentRelation::VertexInterior(t1, t2)
    }
}

/// Split a single polygon's edges at all intersection points with all other polygons.
/// Returns an augmented polygon with intersection points inserted.
fn split_edges_at_intersections(
    contour: &[Point],
    all_contours: &[Vec<Point>],
    contour_idx: usize,
    epsilon: f64,
) -> Vec<Point> {
    let n = contour.len();
    // For each edge of this contour, collect split points
    // structure: for edge i (from contour[i] to contour[i+1]), collect t values where it's split
    let mut split_ts: Vec<Vec<f64>> = vec![Vec::new(); n];

    // Check against every edge of every other contour
    for (other_idx, other_contour) in all_contours.iter().enumerate() {
        if other_idx == contour_idx {
            continue;
        }
        let on = other_contour.len();
        for j in 0..n {
            let p1 = &contour[j];
            let p2 = &contour[(j + 1) % n];
            for k in 0..on {
                let q1 = &other_contour[k];
                let q2 = &other_contour[(k + 1) % on];
                let rel = segment_relation(p1, p2, q1, q2, epsilon);
                match rel {
                    SegmentRelation::ProperInterior(t1, _t2) => {
                        split_ts[j].push(t1);
                    }
                    SegmentRelation::VertexInterior(t1, _t2) => {
                        // Add vertex intersection if it's interior on our edge
                        if t1 > epsilon && t1 < 1.0 - epsilon {
                            split_ts[j].push(t1);
                        }
                    }
                    SegmentRelation::CollinearOverlap { t1_start, t1_end, .. } => {
                        // Collinear overlap: split at both endpoints of the overlap region
                        if t1_start > epsilon && t1_start < 1.0 - epsilon {
                            split_ts[j].push(t1_start);
                        }
                        if t1_end > epsilon && t1_end < 1.0 - epsilon {
                            split_ts[j].push(t1_end);
                        }
                    }
                    SegmentRelation::None => {}
                }
            }
        }
    }

    // Build augmented contour: for each edge, keep start point then insert split points sorted by t
    let mut augmented: Vec<Point> = Vec::new();

    for j in 0..n {
        augmented.push(contour[j]);

        // Sort and deduplicate split t values
        let mut ts = split_ts[j].clone();
        ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // Deduplicate nearby t values
        let mut deduped: Vec<f64> = Vec::new();
        for t in ts {
            if deduped.is_empty() || (t - deduped.last().unwrap()).abs() > epsilon {
                deduped.push(t);
            }
        }

        let p1 = &contour[j];
        let p2 = &contour[(j + 1) % n];
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;

        for t in deduped {
            if t > epsilon && t < 1.0 - epsilon {
                let pt = Point::new(p1.x + dx * t, p1.y + dy * t);
                augmented.push(pt);
            }
        }
    }

    augmented
}

/// Build edge fragments from augmented contours, discarding fragments
/// whose midpoint is inside any other contour.
fn build_visible_fragments(
    all_contours: &[Vec<Point>],
    epsilon: f64,
) -> Vec<EdgeFragment> {
    let mut fragments: Vec<EdgeFragment> = Vec::new();

    for (idx, contour) in all_contours.iter().enumerate() {
        // Split edges at intersections
        let augmented = split_edges_at_intersections(contour, all_contours, idx, epsilon);
        let m = augmented.len();

        // Build fragments from consecutive augmented points
        for j in 0..m {
            let start = augmented[j];
            let end = augmented[(j + 1) % m];

            // Skip zero-length fragments
            if start.roughly_equals(&end, epsilon) {
                continue;
            }

            let mid = start.midpoint(&end);

            // Check if midpoint is inside any other contour
            let mut hidden = false;
            for (other_idx, other_contour) in all_contours.iter().enumerate() {
                if other_idx == idx {
                    continue;
                }
                if point_strictly_inside_polygon(&mid, other_contour, epsilon) {
                    hidden = true;
                    break;
                }
            }

            if !hidden {
                fragments.push(EdgeFragment {
                    start,
                    end,
                    orig_contour_idx: idx,
                });
            }
        }
    }

    fragments
}

/// Deduplicate fragments: remove fragments that have the same start-end pair
/// (within epsilon) as another fragment. For collinear overlapping edges going
/// the same direction, keep only one copy.
fn deduplicate_fragments(fragments: Vec<EdgeFragment>, epsilon: f64) -> Vec<EdgeFragment> {
    let mut result: Vec<EdgeFragment> = Vec::new();
    for frag in fragments {
        // Check if we already have a fragment with the same start and end
        let is_dup = result.iter().any(|existing| {
            existing.start.roughly_equals(&frag.start, epsilon)
                && existing.end.roughly_equals(&frag.end, epsilon)
        });
        if !is_dup {
            result.push(frag);
        }
    }
    result
}

/// Merge collinear adjacent fragments: if one fragment ends where another starts,
/// and they point in the same direction (approximately), merge them.
fn merge_collinear_fragments(fragments: &mut Vec<EdgeFragment>, epsilon: f64) {
    let mut merged = true;
    while merged {
        merged = false;
        let mut i = 0;
        while i < fragments.len() {
            let mut j = i + 1;
            while j < fragments.len() {
                let can_merge = {
                    let a = &fragments[i];
                    let b = &fragments[j];
                    let a_end_to_b_start = a.end.roughly_equals(&b.start, epsilon);
                    let b_end_to_a_start = b.end.roughly_equals(&a.start, epsilon);

                    if a_end_to_b_start || b_end_to_a_start {
                        // Check they are collinear (cross product of directions ~ 0)
                        let da_x = a.end.x - a.start.x;
                        let da_y = a.end.y - a.start.y;
                        let db_x = b.end.x - b.start.x;
                        let db_y = b.end.y - b.start.y;
                        let cross = da_x * db_y - da_y * db_x;
                        let dot = da_x * db_x + da_y * db_y;
                        cross.abs() < epsilon && dot > 0.0
                    } else {
                        false
                    }
                };

                if can_merge {
                    let a = &fragments[i];
                    let b = &fragments[j];
                    let (new_start, new_end) = if a.end.roughly_equals(&b.start, epsilon) {
                        // a->...->b: a.start -> b.end
                        (a.start, b.end)
                    } else {
                        // b->...->a: b.start -> a.end
                        (b.start, a.end)
                    };
                    fragments[i] = EdgeFragment {
                        start: new_start,
                        end: new_end,
                        orig_contour_idx: a.orig_contour_idx,
                    };
                    fragments.remove(j);
                    merged = true;
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }
}

/// Trace fragments into closed contours.
/// Greedy algorithm: start at any unused fragment, follow it, then find the
/// next fragment that starts where the current one ends (within epsilon).
fn trace_contours(fragments: &[EdgeFragment], epsilon: f64) -> Vec<Vec<Point>> {
    let mut used = vec![false; fragments.len()];
    let mut contours: Vec<Vec<Point>> = Vec::new();

    loop {
        // Find first unused fragment
        let start_idx = used.iter().position(|&u| !u);
        if start_idx.is_none() {
            break;
        }
        let start_idx = start_idx.unwrap();

        let mut contour: Vec<Point> = Vec::new();
        let mut current_end = fragments[start_idx].start;

        // Chase fragments
        let mut idx = start_idx;
        loop {
            if used[idx] {
                break;
            }
            used[idx] = true;

            let frag = &fragments[idx];

            // Add start if it's not already the last point
            if contour.is_empty() || !contour.last().unwrap().roughly_equals(&frag.start, epsilon) {
                contour.push(frag.start);
            }
            contour.push(frag.end);

            current_end = frag.end;

            // Find next fragment that starts at current_end
            let next_idx = (0..fragments.len()).find(|&i| {
                !used[i]
                    && fragments[i].start.roughly_equals(&current_end, epsilon)
            });

            match next_idx {
                Some(next) => idx = next,
                None => break,
            }

            // Stop if we've closed the loop
            if current_end.roughly_equals(&contour[0], epsilon) {
                break;
            }
        }

        // Ensure contour is closed
        if contour.len() >= 3 {
            if !contour.last().unwrap().roughly_equals(&contour[0], epsilon) {
                contour.push(contour[0]);
            }
            // Remove duplicate last point if it matches first
            if contour.len() > 1
                && contour.last().unwrap().roughly_equals(&contour[0], epsilon)
            {
                contour.pop();
            }
            contours.push(contour);
        }
    }

    contours
}

/// Remove overlaps from a set of closed contours.
///
/// All contours are assumed to be closed (last point connects to first).
/// Clockwise contours are outer boundaries; counter-clockwise are holes.
///
/// Returns a set of non-overlapping contours that represent the same filled area.
pub fn remove_overlap(contours: &[Vec<Point>]) -> Vec<Vec<Point>> {
    if contours.len() <= 1 {
        return contours.to_vec();
    }

    let epsilon = 1e-6;

    // Step 1: Split edges at intersections and build visible fragments
    let fragments = build_visible_fragments(contours, epsilon);

    if fragments.is_empty() {
        return Vec::new();
    }

    // Step 1b: Deduplicate fragments (collinear overlapping edges)
    let fragments = deduplicate_fragments(fragments, epsilon);

    // Step 1c: Merge collinear adjacent fragments
    let mut fragments = fragments;
    merge_collinear_fragments(&mut fragments, epsilon);

    // Step 2: Trace fragments into closed contours
    let mut result = trace_contours(&fragments, epsilon);

    // Step 3: Filter out degenerate contours (< 3 points)
    result.retain(|c| c.len() >= 3);

    // Step 4: Ensure all output contours are clockwise (outer contours)
    // In the union, all result contours should be outer (clockwise).
    // If any is counter-clockwise, reverse it.
    for contour in result.iter_mut() {
        if !is_clockwise(contour) {
            contour.reverse();
        }
    }

    result
}

/// Compute the axis-aligned bounding box of a set of contours.
pub fn bounding_box(contours: &[Vec<Point>]) -> (f64, f64, f64, f64) {
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

    #[test]
    fn test_point_in_polygon_square() {
        // Unit square
        let square = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        assert!(point_in_polygon(&Point::new(5.0, 5.0), &square));
        assert!(!point_in_polygon(&Point::new(15.0, 5.0), &square));
        assert!(!point_in_polygon(&Point::new(-5.0, 5.0), &square));
    }

    #[test]
    fn test_is_clockwise() {
        let cw = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        let ccw = vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 0.0),
        ];
        assert!(is_clockwise(&cw));
        assert!(!is_clockwise(&ccw));
    }

    #[test]
    fn test_two_overlapping_rectangles_union() {
        // Two overlapping clockwise rectangles (as in the FFI test case)
        let r1 = vec![
            Point::new(0.0, 0.0),
            Point::new(600.0, 0.0),
            Point::new(600.0, 700.0),
            Point::new(0.0, 700.0),
        ];
        let r2 = vec![
            Point::new(200.0, -100.0),
            Point::new(500.0, -100.0),
            Point::new(500.0, 800.0),
            Point::new(200.0, 800.0),
        ];

        let contours = vec![r1, r2];
        let result = remove_overlap(&contours);

        // Result should be 1 outer contour (the union boundary)
        assert_eq!(result.len(), 1, "Expected 1 contour from union, got {}", result.len());

        let union_poly = &result[0];

        // The union should contain points from both rectangles
        assert!(union_poly.len() >= 8, "Expected at least 8 points in union contour");

        // Verify it's clockwise
        assert!(is_clockwise(union_poly), "Union contour should be clockwise");

        // Check bounding box covers both rectangles
        let (min_x, min_y, max_x, max_y) = bounding_box(&result);
        assert!((min_x - 0.0).abs() < 1.0, "min_x should be ~0");
        assert!((min_y - (-100.0)).abs() < 1.0, "min_y should be ~-100");
        assert!((max_x - 600.0).abs() < 1.0, "max_x should be ~600");
        assert!((max_y - 800.0).abs() < 1.0, "max_y should be ~800");
    }

    #[test]
    fn test_disjoint_rectangles_no_union() {
        // Two disjoint rectangles — no overlap to remove
        let r1 = vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
        ];
        let r2 = vec![
            Point::new(200.0, 0.0),
            Point::new(300.0, 0.0),
            Point::new(300.0, 100.0),
            Point::new(200.0, 100.0),
        ];

        let contours = vec![r1, r2];
        let result = remove_overlap(&contours);

        // Two disjoint shapes stay as 2 contours
        assert_eq!(result.len(), 2, "Disjoint rectangles should produce 2 contours");
    }

    #[test]
    fn test_single_contour_passthrough() {
        let square = vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
        ];
        let result = remove_overlap(&[square.clone()]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 4);
    }

    #[test]
    fn test_empty_input() {
        let result = remove_overlap(&[]);
        assert!(result.is_empty());
    }
}
