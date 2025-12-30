//! Gizmo interaction and input handling.
//!
//! This module contains systems for detecting mouse hover over gizmo elements,
//! starting/ending drag operations, and applying transforms during drags.

use bevy::gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore};
use bevy::input::mouse::MouseButton;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Epsilon for zero-length vector checks.
const EPSILON: f32 = 1e-6;

/// Minimum divisor to prevent division by zero in scale calculations.
const MIN_SCALE_DIVISOR: f32 = 1e-3;

use crate::gizmo_frame::{plane_axes, AxisKind, GizmoFrame};
use crate::math::{axis_basis, ray_plane_intersection, ray_sphere_intersection};
use crate::types::{
    GizmoAxis, GizmoOperation, TransformGizmoCamera, TransformGizmoDrag, TransformGizmoSnap,
    TransformGizmoState, TransformGizmoStyle, TransformGizmoTarget,
};

/// Configure Bevy's built-in gizmo renderer using our style resource.
pub fn configure_gizmos(
    mut config_store: ResMut<GizmoConfigStore>,
    style: Res<TransformGizmoStyle>,
) {
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = style.line_width;
    config.depth_bias = style.depth_bias;
}

/// Determine which gizmo part (if any) is currently hovered.
pub fn update_hovered_axis(
    mut state: ResMut<TransformGizmoState>,
    style: Res<TransformGizmoStyle>,
    cameras: Query<(&Camera, &GlobalTransform), With<TransformGizmoCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    targets: Query<(Entity, &GlobalTransform), With<TransformGizmoTarget>>,
) {
    // We only care about hover when we are not currently dragging.
    if state.drag.is_some() {
        return;
    }

    let Some((camera, camera_transform)) = cameras.iter().next() else {
        state.hovered_axis = None;
        state.hovered_op = None;
        return;
    };
    let Some(window) = windows.iter().next() else {
        state.hovered_axis = None;
        state.hovered_op = None;
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        state.hovered_axis = None;
        state.hovered_op = None;
        return;
    };

    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        state.hovered_axis = None;
        state.hovered_op = None;
        return;
    };

    // Search across *all* targets for the closest gizmo element under the cursor.
    let mut best_t = f32::MAX;
    let mut best_target: Option<Entity> = None;
    let mut best: Option<(GizmoOperation, GizmoAxis)> = None;

    for (entity, transform) in targets.iter() {
        let frame = GizmoFrame::new(transform, state.space);
        let origin = frame.origin;

        // Coarse bounds test: if the ray misses the gizmo's bounding sphere
        // sooner than our current best hit, skip this target.
        let Some(bounds_t) = ray_sphere_intersection(&ray, origin, style.bounds_radius) else {
            continue;
        };
        if bounds_t > best_t {
            continue;
        }

        // --- Axis translation cones ---
        let allow_translate = style.show_translate;
        let allow_rotate = style.show_rotate;
        let allow_scale = style.show_scale;

        if allow_translate {
            for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                if !style.translate_axes.enabled(axis) {
                    continue;
                }

                let axis_dir = frame
                    .axis_dir(axis, AxisKind::Translate)
                    .normalize_or_zero();
                if axis_dir.length_squared() < EPSILON {
                    continue;
                }

                // Match the drawn cone: centered between the end of the axis
                // line and the cone tip.
                let line_end = origin + axis_dir * style.axis_length;
                let cone_tip = line_end + axis_dir * style.translate_cone_length;
                let center = (line_end + cone_tip) * 0.5;

                if let Some(t) = ray_sphere_intersection(&ray, center, style.translate_hit_radius) {
                    if t < best_t {
                        best_t = t;
                        best_target = Some(entity);
                        best = Some((GizmoOperation::TranslateAxis, axis));
                    }
                }
            }
        }

        // --- Axis scale cubes ---
        if allow_scale {
            for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                if !style.scale_axes.enabled(axis) {
                    continue;
                }

                let axis_dir = frame.axis_dir(axis, AxisKind::Scale).normalize_or_zero();
                if axis_dir.length_squared() < EPSILON {
                    continue;
                }

                let center = origin + axis_dir * (style.axis_length * style.scale_cube_offset);

                if let Some(t) = ray_sphere_intersection(&ray, center, style.scale_hit_radius) {
                    if t < best_t {
                        best_t = t;
                        best_target = Some(entity);
                        best = Some((GizmoOperation::ScaleAxis, axis));
                    }
                }
            }
        }

        // --- Rotation arcs ---
        if allow_rotate {
            for (axis, axis_vec, n1, n2) in [
                (
                    GizmoAxis::X,
                    frame.axis_dir(GizmoAxis::X, AxisKind::Rotate),
                    frame.axis_dir(GizmoAxis::Y, AxisKind::Rotate),
                    frame.axis_dir(GizmoAxis::Z, AxisKind::Rotate),
                ),
                (
                    GizmoAxis::Y,
                    frame.axis_dir(GizmoAxis::Y, AxisKind::Rotate),
                    frame.axis_dir(GizmoAxis::Z, AxisKind::Rotate),
                    frame.axis_dir(GizmoAxis::X, AxisKind::Rotate),
                ),
                (
                    GizmoAxis::Z,
                    frame.axis_dir(GizmoAxis::Z, AxisKind::Rotate),
                    frame.axis_dir(GizmoAxis::X, AxisKind::Rotate),
                    frame.axis_dir(GizmoAxis::Y, AxisKind::Rotate),
                ),
            ] {
                if !style.rotate_axes.enabled(axis) {
                    continue;
                }

                let axis_dir = axis_vec.normalize_or_zero();
                if axis_dir.length_squared() < EPSILON {
                    continue;
                }

                let Some(hit_point) = crate::math::ray_plane_intersection(&ray, origin, axis_dir)
                else {
                    continue;
                };

                let v = hit_point - origin;
                let radius = v.length();
                if radius < 1e-4 {
                    continue;
                }

                let ring_radius = style.axis_length;
                if (radius - ring_radius).abs() > style.rotation_hit_thickness {
                    continue;
                }

                let (t1, t2) = axis_basis(axis_dir);
                let proj = v.normalize_or_zero();
                let x = proj.dot(t1);
                let y = proj.dot(t2);
                let angle = y.atan2(x);

                let mid = (n1 + n2).normalize_or_zero();
                let mid = mid - axis_dir * axis_dir.dot(mid);
                let mid = mid.normalize_or_zero();
                let mx = mid.dot(t1);
                let my = mid.dot(t2);
                let centre = my.atan2(mx);

                let half = style.rotation_arc_degrees.to_radians() * 0.5;
                let diff = (angle - centre + std::f32::consts::PI)
                    .rem_euclid(2.0 * std::f32::consts::PI)
                    - std::f32::consts::PI;

                if diff.abs() > half {
                    continue;
                }

                if let Some(t) =
                    ray_sphere_intersection(&ray, hit_point, style.rotation_hit_thickness)
                {
                    if t < best_t {
                        best_t = t;
                        best_target = Some(entity);
                        best = Some((GizmoOperation::Rotate, axis));
                    }
                }
            }
        }

        // --- Planar translation rectangles ---
        if allow_translate && style.show_translate_planes {
            for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                if !style.translate_axes.enabled(axis) {
                    continue;
                }
                let (d1_axis, d2_axis) = plane_axes(axis);

                let plane_normal = frame
                    .axis_dir(axis, AxisKind::Translate)
                    .normalize_or_zero();
                let dir1 = frame
                    .axis_dir(d1_axis, AxisKind::Translate)
                    .normalize_or_zero();
                let dir2 = frame
                    .axis_dir(d2_axis, AxisKind::Translate)
                    .normalize_or_zero();
                if plane_normal.length_squared() < EPSILON
                    || dir1.length_squared() < EPSILON
                    || dir2.length_squared() < EPSILON
                {
                    continue;
                }

                let Some(hit_point) =
                    crate::math::ray_plane_intersection(&ray, origin, plane_normal)
                else {
                    continue;
                };

                let local = hit_point - origin;
                let u = local.dot(dir1);
                let v = local.dot(dir2);

                let offset = style.translate_plane_offset;
                let size = style.translate_plane_size;
                let pad = style.translate_plane_hit_thickness;

                let inside = u >= offset - pad
                    && u <= offset + size + pad
                    && v >= offset - pad
                    && v <= offset + size + pad;

                if inside {
                    let t = (hit_point - ray.origin).dot(*ray.direction);
                    if t >= 0.0 && t < best_t {
                        best_t = t;
                        best_target = Some(entity);
                        best = Some((GizmoOperation::TranslatePlane, axis));
                    }
                }
            }
        }
        // --- Uniform scale square at the origin ---
        if allow_scale && style.show_scale_uniform {
            // Treat the uniform scale handle as a small sphere around the origin.
            if let Some(t) = ray_sphere_intersection(&ray, origin, style.scale_uniform_hit_radius) {
                if t < best_t {
                    best_t = t;
                    best_target = Some(entity);
                    // Axis is unused for uniform scale, but we must provide one.
                    best = Some((GizmoOperation::ScaleUniform, GizmoAxis::X));
                }
            }
        }
    }

    if let (Some(target), Some((op, axis))) = (best_target, best) {
        state.active_target = Some(target);
        state.hovered_axis = Some(axis);
        state.hovered_op = Some(op);
    } else {
        state.hovered_axis = None;
        state.hovered_op = None;
    }
}
pub fn begin_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<TransformGizmoState>,
    cameras: Query<(&Camera, &GlobalTransform), With<TransformGizmoCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    targets: Query<(Entity, &GlobalTransform, &mut Transform), With<TransformGizmoTarget>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    if state.drag.is_some() {
        return;
    }

    let Some(axis) = state.hovered_axis else {
        return;
    };
    let Some(op) = state.hovered_op else {
        return;
    };

    let Some((camera, camera_transform)) = cameras.iter().next() else {
        return;
    };
    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };

    let Some(target_entity) = state.active_target else {
        return;
    };
    let Ok((entity, global, _local_transform)) = targets.get(target_entity) else {
        return;
    };

    let frame = GizmoFrame::new(global, state.space);
    let origin = frame.origin;

    // Axis direction or plane normal depending on operation.
    let axis_vec = match op {
        GizmoOperation::TranslateAxis | GizmoOperation::TranslatePlane => {
            frame.axis_dir(axis, AxisKind::Translate)
        }
        GizmoOperation::Rotate => frame.axis_dir(axis, AxisKind::Rotate),
        GizmoOperation::ScaleAxis => frame.axis_dir(axis, AxisKind::Scale),
        GizmoOperation::ScaleUniform => *camera_transform.forward(),
    };
    let axis_dir = axis_vec.normalize_or_zero();

    // Plane normal used to project mouse movement.
    let plane_normal = match op {
        GizmoOperation::Rotate => axis_dir,
        GizmoOperation::TranslateAxis | GizmoOperation::ScaleAxis => {
            // Plane that is perpendicular to both axis and camera view.
            let view_dir: Vec3 = -*camera_transform.forward();
            let n = axis_dir.cross(view_dir).cross(axis_dir).normalize_or_zero();
            if n.length_squared() < EPSILON {
                axis_dir
            } else {
                n
            }
        }
        GizmoOperation::TranslatePlane => {
            // Movement constrained to a fixed plane: use the plane normal directly.
            axis_dir
        }
        GizmoOperation::ScaleUniform => {
            // For uniform scale, use a plane whose normal is perpendicular to
            // the view direction so that mouse motion produces a sensible
            // distance change.
            let view_dir: Vec3 = -*camera_transform.forward();
            let helper = if view_dir.abs().dot(Vec3::Y) < 0.9 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let n = view_dir.cross(helper).normalize_or_zero();
            if n.length_squared() < EPSILON {
                view_dir
            } else {
                n
            }
        }
    };

    // For planar translation, intersect the ray with the plane that passes
    // through the "L" corner so that the handle stays under the cursor.
    let plane_origin = origin;
    let mut plane_dir1 = Vec3::ZERO;
    let mut plane_dir2 = Vec3::ZERO;
    let mut plane_axis1 = GizmoAxis::X;
    let mut plane_axis2 = GizmoAxis::Y;

    let hit_point = ray_plane_intersection(&ray, plane_origin, plane_normal).unwrap_or(origin);
    let v = hit_point - origin;

    let start_t = match op {
        GizmoOperation::TranslateAxis | GizmoOperation::ScaleAxis => v.dot(axis_dir),
        GizmoOperation::Rotate => {
            // Angle around axis.
            let (t1, t2) = axis_basis(axis_dir);
            let proj = v.normalize_or_zero();
            let x = proj.dot(t1);
            let y = proj.dot(t2);
            y.atan2(x)
        }
        GizmoOperation::TranslatePlane => 0.0,
        GizmoOperation::ScaleUniform => {
            // Distance along camera forward.
            v.length()
        }
    };

    let start_vector = match op {
        GizmoOperation::TranslatePlane => {
            let (a1, a2) = plane_axes(axis);
            plane_axis1 = a1;
            plane_axis2 = a2;
            plane_dir1 = frame.axis_dir(a1, AxisKind::Translate).normalize_or_zero();
            plane_dir2 = frame.axis_dir(a2, AxisKind::Translate).normalize_or_zero();

            let n = plane_normal;
            v - n * v.dot(n)
        }
        GizmoOperation::Rotate => v,
        _ => Vec3::ZERO,
    };

    state.drag = Some(TransformGizmoDrag {
        target: entity,
        op,
        axis,
        origin,
        axis_dir,
        plane_normal,
        plane_origin,
        plane_dir1,
        plane_dir2,
        plane_axis1,
        plane_axis2,
        start_translation: global.translation(),
        start_rotation: global.rotation(),
        start_scale: global.to_scale_rotation_translation().0,
        start_t,
        start_vector,
    });
}

/// Update the drag operation while the mouse is held down.
pub fn drag_gizmo(
    buttons: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<TransformGizmoState>,
    snap: Res<TransformGizmoSnap>,
    cameras: Query<(&Camera, &GlobalTransform), With<TransformGizmoCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut targets: Query<&mut Transform, With<TransformGizmoTarget>>,
) {
    let Some(drag) = state.drag.as_mut() else {
        return;
    };

    if !buttons.pressed(MouseButton::Left) {
        return;
    }

    let Some((camera, camera_transform)) = cameras.iter().next() else {
        return;
    };
    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };

    let Ok(mut transform) = targets.get_mut(drag.target) else {
        return;
    };

    let hit_point =
        ray_plane_intersection(&ray, drag.plane_origin, drag.plane_normal).unwrap_or(drag.origin);
    let v = hit_point - drag.origin;

    match drag.op {
        GizmoOperation::TranslateAxis => {
            let t = v.dot(drag.axis_dir);
            let mut delta = t - drag.start_t;
            if let Some(step) = snap.translate.get(drag.axis) {
                if step > 0.0 {
                    delta = (delta / step).round() * step;
                }
            }
            transform.translation = drag.start_translation + delta * drag.axis_dir;
        }
        GizmoOperation::TranslatePlane => {
            let n = drag.plane_normal;
            let proj = v - n * v.dot(n);
            let mut delta = proj - drag.start_vector;

            // Snap along the two plane axes independently.
            let mut u = delta.dot(drag.plane_dir1);
            let mut w = delta.dot(drag.plane_dir2);
            if let Some(step) = snap.translate.get(drag.plane_axis1) {
                if step > 0.0 {
                    u = (u / step).round() * step;
                }
            }
            if let Some(step) = snap.translate.get(drag.plane_axis2) {
                if step > 0.0 {
                    w = (w / step).round() * step;
                }
            }
            delta = drag.plane_dir1 * u + drag.plane_dir2 * w;

            transform.translation = drag.start_translation + delta;
        }
        GizmoOperation::ScaleAxis => {
            let t = v.dot(drag.axis_dir);
            // Guard against division by zero when start_t is near zero
            let delta = (t - drag.start_t) / drag.start_t.max(MIN_SCALE_DIVISOR);
            let mut scale = drag.start_scale;
            match drag.axis {
                GizmoAxis::X => scale.x *= snap_scale(scale.x, delta, snap.scale.get(GizmoAxis::X)),
                GizmoAxis::Y => scale.y *= snap_scale(scale.y, delta, snap.scale.get(GizmoAxis::Y)),
                GizmoAxis::Z => scale.z *= snap_scale(scale.z, delta, snap.scale.get(GizmoAxis::Z)),
            }
            transform.scale = scale;
        }
        GizmoOperation::ScaleUniform => {
            let t = v.length();
            let factor = if drag.start_t.abs() > MIN_SCALE_DIVISOR {
                t / drag.start_t
            } else {
                1.0
            };
            let base = drag.start_scale;
            let snap_step = snap.scale.get(GizmoAxis::X).unwrap_or(0.0);
            let snapped_factor = if snap_step > 0.0 {
                let target = base.x * factor;
                let snapped = (target / snap_step).round() * snap_step;
                if base.x.abs() > EPSILON {
                    snapped / base.x
                } else {
                    factor
                }
            } else {
                factor
            };
            transform.scale = base * snapped_factor.max(0.001);
        }
        GizmoOperation::Rotate => {
            let (t1, t2) = axis_basis(drag.axis_dir);
            let proj = v.normalize_or_zero();
            let x = proj.dot(t1);
            let y = proj.dot(t2);
            let angle = y.atan2(x);
            let mut delta_angle = angle - drag.start_t;
            if let Some(step) = snap.rotate.get(drag.axis) {
                if step > 0.0 {
                    delta_angle = (delta_angle / step).round() * step;
                }
            }
            let delta_rot = Quat::from_axis_angle(drag.axis_dir, delta_angle);
            transform.rotation = delta_rot * drag.start_rotation;
        }
    }
}

fn snap_scale(base: f32, delta: f32, step: Option<f32>) -> f32 {
    let raw = 1.0 + delta;
    match step {
        Some(step) if step > 0.0 => {
            let target = base * raw;
            let snapped = (target / step).round() * step;
            if base.abs() > EPSILON {
                snapped / base
            } else {
                raw
            }
        }
        _ => raw,
    }
}

/// End the drag operation when the mouse button is released.
pub fn end_drag(buttons: Res<ButtonInput<MouseButton>>, mut state: ResMut<TransformGizmoState>) {
    if buttons.just_released(MouseButton::Left) {
        state.drag = None;
    }
}
