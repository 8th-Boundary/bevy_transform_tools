//! Math utilities for gizmo hit testing and intersection calculations.

use bevy::math::Ray3d;
use bevy::prelude::*;

/// Threshold for considering vectors as parallel or zero-length.
const EPSILON: f32 = 1e-6;

/// Threshold for parallel plane/ray detection.
const PLANE_EPSILON: f32 = 1e-5;

/// Threshold for choosing perpendicular helper vector.
const AXIS_PARALLEL_THRESHOLD: f32 = 0.9;

/// Build an orthonormal basis (t1, t2) in the plane perpendicular to `axis`.
pub fn axis_basis(axis: Vec3) -> (Vec3, Vec3) {
    let axis = axis.normalize_or_zero();
    if axis.length_squared() < EPSILON {
        return (Vec3::X, Vec3::Y);
    }

    // Pick a helper vector that is not parallel to axis.
    let helper = if axis.abs().dot(Vec3::Y) < AXIS_PARALLEL_THRESHOLD {
        Vec3::Y
    } else {
        Vec3::X
    };

    let t1 = axis.cross(helper).normalize_or_zero();
    let t2 = axis.cross(t1).normalize_or_zero();
    (t1, t2)
}

/// Solve intersection between a ray and a sphere. Returns distance along the
/// ray if there is an intersection, otherwise `None`.
pub fn ray_sphere_intersection(ray: &Ray3d, center: Vec3, radius: f32) -> Option<f32> {
    let m = ray.origin - center;
    let b = m.dot(*ray.direction);
    let c = m.length_squared() - radius * radius;

    // Exit if ray origin is outside sphere (c > 0) and ray is pointing away
    // from sphere (b > 0).
    if c > 0.0 && b > 0.0 {
        return None;
    }

    let discr = b * b - c;
    if discr < 0.0 {
        return None;
    }

    let t = -b - discr.sqrt();
    if t < 0.0 {
        Some(0.0)
    } else {
        Some(t)
    }
}

/// Intersect a ray with a plane. Returns the intersection point, if any.
pub fn ray_plane_intersection(ray: &Ray3d, plane_origin: Vec3, plane_normal: Vec3) -> Option<Vec3> {
    let denom = plane_normal.dot(*ray.direction);
    if denom.abs() < PLANE_EPSILON {
        return None;
    }
    let t = (plane_origin - ray.origin).dot(plane_normal) / denom;
    if t < 0.0 {
        None
    } else {
        Some(ray.origin + *ray.direction * t)
    }
}
