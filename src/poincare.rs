//! Poincaré ball model — distance, exponential map, and Riemannian SGD.
//! Curvature c = 1.0 (unit ball).
//!
//! # Examples — basic norm and ball invariants
//!
//! ```
//! use open_ontologies::poincare::{norm, poincare_distance, project_to_ball};
//!
//! // The origin has norm 0.0
//! assert!((norm(&[0.0_f32, 0.0]) - 0.0).abs() < 1e-6);
//!
//! // A unit vector has norm 1.0
//! assert!((norm(&[1.0_f32, 0.0]) - 1.0).abs() < 1e-6);
//!
//! // Any valid Poincaré point must lie strictly inside the unit ball
//! let p = [0.3_f32, 0.4];
//! assert!(norm(&p) < 1.0, "valid point must be inside the ball");
//!
//! // Projecting an outside point brings it inside
//! let outside = [0.8_f32, 0.8];
//! let inside = project_to_ball(&outside, 1e-5);
//! assert!(norm(&inside) < 1.0, "projected point must be inside the ball");
//!
//! // Distance from any point to itself is 0
//! let u = [0.1_f32, 0.2];
//! assert!((poincare_distance(&u, &u) - 0.0).abs() < 1e-5);
//! ```

const EPS: f32 = 1e-5;

/// Euclidean L2 norm of a vector (pre-check before Poincaré operations).
///
/// A point is inside the unit Poincaré ball if and only if `norm(p) < 1.0`.
///
/// # Examples
///
/// ```
/// use open_ontologies::poincare::norm;
///
/// // Origin has norm 0.0
/// assert!((norm(&[0.0_f32, 0.0]) - 0.0).abs() < 1e-6);
///
/// // 3-4-5 right triangle: norm of [3, 4] is 5
/// assert!((norm(&[3.0_f32, 4.0]) - 5.0).abs() < 1e-5);
///
/// // Boundary condition: any valid Poincaré point has norm < 1.0
/// let valid = [0.3_f32, 0.4]; // norm = 0.5 < 1
/// assert!(norm(&valid) < 1.0);
///
/// // A point on or beyond the boundary has norm >= 1.0
/// let boundary = [1.0_f32, 0.0];
/// assert!(norm(&boundary) >= 1.0 - 1e-5);
/// ```
pub fn norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

/// Poincaré ball distance: d(u,v) = arcosh(1 + 2||u-v||² / ((1-||u||²)(1-||v||²)))
///
/// # Examples
///
/// ```
/// use open_ontologies::poincare::poincare_distance;
///
/// // Distance from a point to itself is 0
/// let u = [0.1_f32, 0.2];
/// assert!((poincare_distance(&u, &u) - 0.0).abs() < 1e-5);
///
/// // Distance is symmetric
/// let v = [0.3_f32, 0.1];
/// assert!((poincare_distance(&u, &v) - poincare_distance(&v, &u)).abs() < 1e-5);
///
/// // Distance is non-negative
/// assert!(poincare_distance(&u, &v) >= 0.0);
/// ```
pub fn poincare_distance(u: &[f32], v: &[f32]) -> f32 {
    let diff_sq: f32 = u.iter().zip(v.iter()).map(|(a, b)| (a - b).powi(2)).sum();
    let norm_u_sq: f32 = u.iter().map(|x| x * x).sum();
    let norm_v_sq: f32 = v.iter().map(|x| x * x).sum();
    let denom = (1.0 - norm_u_sq).max(EPS) * (1.0 - norm_v_sq).max(EPS);
    let x = 1.0 + 2.0 * diff_sq / denom;
    let x = x.max(1.0);
    (x + (x * x - 1.0).max(0.0).sqrt()).ln()
}

/// Cosine similarity between two vectors.
///
/// # Examples
///
/// ```
/// use open_ontologies::poincare::cosine_similarity;
///
/// // A vector has cosine similarity 1.0 with itself
/// let a = [1.0_f32, 0.0, 0.0];
/// assert!((cosine_similarity(&a, &a) - 1.0).abs() < 1e-5);
///
/// // Orthogonal vectors have cosine similarity 0.0
/// let b = [0.0_f32, 1.0, 0.0];
/// assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-5);
///
/// // Zero vector returns 0.0 (guarded against division by zero)
/// let zero = [0.0_f32, 0.0, 0.0];
/// assert_eq!(cosine_similarity(&zero, &a), 0.0);
/// ```
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < EPS || norm_b < EPS {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Conformal factor λ_x = 2 / (1 - ||x||²)
fn conformal_factor(x: &[f32]) -> f32 {
    let norm_sq: f32 = x.iter().map(|v| v * v).sum();
    2.0 / (1.0 - norm_sq).max(EPS)
}

/// Exponential map: maps a tangent vector at x to a point on the Poincaré ball.
///
/// # Examples
///
/// ```
/// use open_ontologies::poincare::exp_map;
///
/// // Zero tangent vector returns the base point unchanged
/// let x = [0.1_f32, 0.2];
/// let zero = [0.0_f32, 0.0];
/// let result = exp_map(&x, &zero);
/// assert!((result[0] - x[0]).abs() < 1e-5);
/// assert!((result[1] - x[1]).abs() < 1e-5);
///
/// // Result stays inside the unit ball (norm < 1)
/// let v = [0.5_f32, 0.5];
/// let mapped = exp_map(&x, &v);
/// let norm: f32 = mapped.iter().map(|c| c * c).sum::<f32>().sqrt();
/// assert!(norm < 1.0);
/// ```
pub fn exp_map(x: &[f32], v: &[f32]) -> Vec<f32> {
    let norm_v: f32 = v.iter().map(|a| a * a).sum::<f32>().sqrt();
    if norm_v < EPS {
        return x.to_vec();
    }
    let lambda = conformal_factor(x);
    let t = (lambda * norm_v / 2.0).tanh();
    let direction: Vec<f32> = v.iter().map(|a| t * a / norm_v).collect();
    let result = mobius_add(x, &direction);
    project_to_ball(&result, EPS)
}

/// Möbius addition: x ⊕ y
fn mobius_add(x: &[f32], y: &[f32]) -> Vec<f32> {
    let x_dot_y: f32 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
    let norm_x_sq: f32 = x.iter().map(|a| a * a).sum();
    let norm_y_sq: f32 = y.iter().map(|a| a * a).sum();
    let denom = 1.0 + 2.0 * x_dot_y + norm_x_sq * norm_y_sq;
    let denom = denom.max(EPS);
    let num_x = 1.0 + 2.0 * x_dot_y + norm_y_sq;
    let num_y = 1.0 - norm_x_sq;
    x.iter()
        .zip(y.iter())
        .map(|(xi, yi)| (num_x * xi + num_y * yi) / denom)
        .collect()
}

/// Project point back into the Poincaré ball (clamp norm < 1 - eps).
///
/// # Examples
///
/// ```
/// use open_ontologies::poincare::project_to_ball;
///
/// // A point outside the ball is projected inside
/// let outside = [1.0_f32, 1.0, 1.0];
/// let projected = project_to_ball(&outside, 1e-5);
/// let norm: f32 = projected.iter().map(|x| x * x).sum::<f32>().sqrt();
/// assert!(norm < 1.0, "projected norm {norm} must be inside the ball");
///
/// // A point already inside the ball is returned unchanged
/// let inside = [0.1_f32, 0.2, 0.0];
/// let unchanged = project_to_ball(&inside, 1e-5);
/// assert!((unchanged[0] - inside[0]).abs() < 1e-6);
/// assert!((unchanged[1] - inside[1]).abs() < 1e-6);
/// ```
pub fn project_to_ball(p: &[f32], eps: f32) -> Vec<f32> {
    let norm: f32 = p.iter().map(|x| x * x).sum::<f32>().sqrt();
    let max_norm = 1.0 - eps;
    if norm >= max_norm {
        let scale = max_norm / norm;
        p.iter().map(|x| x * scale).collect()
    } else {
        p.to_vec()
    }
}

/// Riemannian SGD step on the Poincaré ball.
/// Rescales Euclidean gradient by (1 - ||x||²)² / 4, then applies exp_map.
///
/// # Examples
///
/// ```
/// use open_ontologies::poincare::rsgd_step;
///
/// // Result of an RSGD step stays inside the unit ball
/// let point = [0.1_f32, 0.2];
/// let grad = [1.0_f32, 0.5];
/// let updated = rsgd_step(&point, &grad, 0.01);
/// let norm: f32 = updated.iter().map(|x| x * x).sum::<f32>().sqrt();
/// assert!(norm < 1.0, "RSGD result must remain in the ball, got norm {norm}");
///
/// // Zero learning rate leaves the point unchanged
/// let same = rsgd_step(&point, &grad, 0.0);
/// assert!((same[0] - point[0]).abs() < 1e-5);
/// assert!((same[1] - point[1]).abs() < 1e-5);
/// ```
pub fn rsgd_step(point: &[f32], euclidean_grad: &[f32], lr: f32) -> Vec<f32> {
    let norm_sq: f32 = point.iter().map(|x| x * x).sum();
    let scale = ((1.0 - norm_sq).max(EPS)).powi(2) / 4.0;
    let tangent: Vec<f32> = euclidean_grad.iter().map(|g| -lr * scale * g).collect();
    exp_map(point, &tangent)
}

/// L2-normalize a vector (project onto unit sphere for cosine space).
///
/// # Examples
///
/// ```
/// use open_ontologies::poincare::l2_normalize;
///
/// // Normalized vector has unit length
/// let v = [3.0_f32, 4.0];
/// let normed = l2_normalize(&v);
/// let norm: f32 = normed.iter().map(|x| x * x).sum::<f32>().sqrt();
/// assert!((norm - 1.0).abs() < 1e-5);
///
/// // Direction is preserved: normed = v / ||v||
/// assert!((normed[0] - 0.6).abs() < 1e-5); // 3/5
/// assert!((normed[1] - 0.8).abs() < 1e-5); // 4/5
///
/// // Zero vector is returned as-is (no division by zero)
/// let zero = [0.0_f32, 0.0];
/// let normed_zero = l2_normalize(&zero);
/// assert_eq!(normed_zero, vec![0.0_f32, 0.0]);
/// ```
pub fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < EPS {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_triangle_inequality() {
        let a = vec![0.1, 0.2];
        let b = vec![0.3, -0.1];
        let c = vec![-0.2, 0.4];
        let d_ab = poincare_distance(&a, &b);
        let d_bc = poincare_distance(&b, &c);
        let d_ac = poincare_distance(&a, &c);
        assert!(d_ac <= d_ab + d_bc + 1e-5, "Triangle inequality violated");
    }

    #[test]
    fn mobius_add_identity() {
        let origin = vec![0.0, 0.0, 0.0];
        let p = vec![0.3, 0.4, 0.0];
        let result = mobius_add(&origin, &p);
        for (a, b) in result.iter().zip(p.iter()) {
            assert!((a - b).abs() < 1e-5, "Origin should be identity for Mobius addition");
        }
    }

    #[test]
    fn conformal_factor_at_origin() {
        let origin = vec![0.0, 0.0];
        let lambda = conformal_factor(&origin);
        assert!((lambda - 2.0).abs() < 1e-5, "Conformal factor at origin should be 2.0, got {}", lambda);
    }

    #[test]
    fn conformal_factor_increases_near_boundary() {
        let near_center = vec![0.1, 0.0];
        let near_edge = vec![0.9, 0.0];
        let lc = conformal_factor(&near_center);
        let le = conformal_factor(&near_edge);
        assert!(le > lc, "Conformal factor should increase near boundary");
    }
}
