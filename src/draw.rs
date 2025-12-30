//! Gizmo rendering systems.
//!
//! This module handles drawing the visual representation of the transform
//! gizmo using Bevy's `Gizmos` API.

use std::f32::consts::PI;

use bevy::prelude::*;

/// Number of line segments used to draw translation cones.
const CONE_SEGMENTS: usize = 16;

use crate::gizmo_frame::{plane_axes, AxisKind, GizmoFrame};
use crate::math::axis_basis;
use crate::types::{
    AxisColors, GizmoAxis, GizmoOperation, TransformGizmoCamera, TransformGizmoState,
    TransformGizmoStyle, TransformGizmoTarget,
};

/// Which axis lines should visually respond to a handle interaction.
fn axes_involved(op: GizmoOperation, axis: GizmoAxis) -> Vec<GizmoAxis> {
    match op {
        GizmoOperation::TranslateAxis | GizmoOperation::ScaleAxis => vec![axis],
        GizmoOperation::TranslatePlane => {
            let (a, b) = plane_axes(axis);
            vec![a, b]
        }
        GizmoOperation::Rotate => vec![axis],
        GizmoOperation::ScaleUniform => {
            vec![GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z]
        }
    }
}

/// Determine whether a given (operation, axis) is currently active (being dragged).
fn is_axis_active(
    state: &TransformGizmoState,
    target: Entity,
    op: GizmoOperation,
    axis: GizmoAxis,
) -> bool {
    if let Some(drag) = &state.drag {
        drag.target == target && drag.op == op && drag.axis == axis
    } else {
        false
    }
}

struct GizmoDrawContext<'a> {
    state: &'a TransformGizmoState,
    style: &'a TransformGizmoStyle,
    frame: &'a GizmoFrame,
    target: Entity,
    hover_axes: Vec<GizmoAxis>,
    active_axes: Vec<GizmoAxis>,
}

impl<'a> GizmoDrawContext<'a> {
    fn color(&self, group: &AxisColors, axis: GizmoAxis, op: GizmoOperation) -> Color {
        gizmo_display_color(self.state, self.target, group, axis, op)
    }

    fn axis_line_color(&self, axis: GizmoAxis) -> Color {
        let colors = self.style.axis_lines.for_axis(axis);
        let is_active = self.active_axes.contains(&axis);
        let is_hovered = self.hover_axes.contains(&axis);

        if is_active {
            colors.active
        } else if is_hovered {
            colors.hover
        } else {
            colors.idle
        }
    }
}

/// Lookup the display color for a gizmo element based on the style and state.
fn gizmo_display_color(
    state: &TransformGizmoState,
    target: Entity,
    group: &AxisColors,
    axis: GizmoAxis,
    op: GizmoOperation,
) -> Color {
    let colors = group.for_axis(axis);
    let is_active = is_axis_active(state, target, op, axis);
    let is_hovered = state.active_target == Some(target)
        && state.hovered_axis == Some(axis)
        && state.hovered_op == Some(op);

    if is_active {
        colors.active
    } else if is_hovered {
        colors.hover
    } else {
        colors.idle
    }
}

/// Draw rotation arc for a given axis using an explicit center angle and basis.
///
/// The arc is drawn between the two other axes (e.g. the X-rotation ring lies
/// in the YZ plane, roughly between the +Y and +Z axes).
#[allow(clippy::too_many_arguments)]
fn draw_rotation_arc(
    gizmos: &mut Gizmos,
    origin: Vec3,
    axis_dir: Vec3,
    neighbor1_dir: Vec3,
    neighbor2_dir: Vec3,
    color: Color,
    radius: f32,
    total_angle_radians: f32,
    segments: usize,
) {
    let axis_dir = axis_dir.normalize_or_zero();
    if axis_dir.length_squared() < 1e-6 {
        return;
    }

    // Build an orthonormal basis in the rotation plane.
    let (t1, t2) = axis_basis(axis_dir);

    // Project neighbors into the rotation plane.
    let proj = |v: Vec3| {
        let v = v.normalize_or_zero();
        let n = axis_dir * axis_dir.dot(v);
        (v - n).normalize_or_zero()
    };
    let n1 = proj(neighbor1_dir);
    let n2 = proj(neighbor2_dir);

    let mid = (n1 + n2).normalize_or_zero();
    let center_angle = if mid.length_squared() > 1e-6 {
        let x = mid.dot(t1);
        let y = mid.dot(t2);
        y.atan2(x)
    } else {
        // Fallback: arbitrary direction in the plane.
        0.0
    };

    let half_angle = total_angle_radians * 0.5;
    let start_angle = center_angle - half_angle;
    let end_angle = center_angle + half_angle;

    let steps = segments.max(2);

    let mut prev_point: Option<Vec3> = None;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        let dir_in_plane = t1 * angle.cos() + t2 * angle.sin();
        let point = origin + radius * dir_in_plane;

        if let Some(prev) = prev_point {
            gizmos.line(prev, point, color);
        }
        prev_point = Some(point);
    }
}

/// Draw a small camera-facing cross (used for the origin dot).
fn draw_origin_dot(
    gizmos: &mut Gizmos,
    origin: Vec3,
    size: f32,
    color: Color,
    camera_transform: &GlobalTransform,
) {
    let right: Vec3 = camera_transform.right().into();
    let up: Vec3 = camera_transform.up().into();
    let half = size * 0.5;

    // Nudge the origin dot slightly toward the camera so it renders clearly
    // on top of other gizmo elements.
    let forward = camera_transform.forward();
    let origin = origin - *forward * 0.02;

    let d1 = (right + up).normalize_or_zero() * half;
    let d2 = (right - up).normalize_or_zero() * half;
    gizmos.line(origin - d1, origin + d1, color);
    gizmos.line(origin - d2, origin + d2, color);
}

/// Draw a camera-facing square at the origin (uniform scale handle).
fn draw_uniform_scale_square(
    gizmos: &mut Gizmos,
    origin: Vec3,
    size: f32,
    color: Color,
    camera_transform: &GlobalTransform,
) {
    let right: Vec3 = camera_transform.right().into();
    let up: Vec3 = camera_transform.up().into();
    let half = size * 0.5;

    let r = right * half;
    let u = up * half;

    let p0 = origin - r - u;
    let p1 = origin + r - u;
    let p2 = origin + r + u;
    let p3 = origin - r + u;

    gizmos.line(p0, p1, color);
    gizmos.line(p1, p2, color);
    gizmos.line(p2, p3, color);
    gizmos.line(p3, p0, color);
}

fn draw_axis_lines(ctx: &GizmoDrawContext, gizmos: &mut Gizmos, axis_length: f32) {
    for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
        let dir = ctx
            .frame
            .axis_dir(axis, AxisKind::Translate)
            .normalize_or_zero();
        if dir.length_squared() < 1e-6 {
            continue;
        }

        let color = ctx.axis_line_color(axis);
        let end = ctx.frame.origin + dir * axis_length;
        gizmos.line(ctx.frame.origin, end, color);
    }
}

fn draw_translation_cones(ctx: &GizmoDrawContext, gizmos: &mut Gizmos, axis_length: f32) {
    for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
        if !ctx.style.translate_axes.enabled(axis) {
            continue;
        }
        let axis_dir = ctx
            .frame
            .axis_dir(axis, AxisKind::Translate)
            .normalize_or_zero();
        if axis_dir.length_squared() < 1e-6 {
            continue;
        }

        let color = ctx.color(&ctx.style.translate, axis, GizmoOperation::TranslateAxis);

        let line_end = ctx.frame.origin + axis_dir * axis_length;
        let cone_tip = line_end + axis_dir * ctx.style.translate_cone_length;

        let (t1, t2) = axis_basis(axis_dir);
        for i in 0..CONE_SEGMENTS {
            let a0 = 2.0 * PI * (i as f32) / (CONE_SEGMENTS as f32);
            let a1 = 2.0 * PI * (i as f32 + 1.0) / (CONE_SEGMENTS as f32);
            let dir0 = t1 * a0.cos() + t2 * a0.sin();
            let dir1 = t1 * a1.cos() + t2 * a1.sin();

            let base0 = line_end + dir0 * ctx.style.translate_cone_radius;
            let base1 = line_end + dir1 * ctx.style.translate_cone_radius;

            gizmos.line(cone_tip, base0, color);
            gizmos.line(cone_tip, base1, color);
            gizmos.line(base0, base1, color);
        }
    }
}

fn draw_translation_planes(ctx: &GizmoDrawContext, gizmos: &mut Gizmos) {
    for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
        if !ctx.style.translate_axes.enabled(axis) {
            continue;
        }
        let (d1_axis, d2_axis) = plane_axes(axis);

        let n = ctx
            .frame
            .axis_dir(axis, AxisKind::Translate)
            .normalize_or_zero();
        let dir1 = ctx
            .frame
            .axis_dir(d1_axis, AxisKind::Translate)
            .normalize_or_zero();
        let dir2 = ctx
            .frame
            .axis_dir(d2_axis, AxisKind::Translate)
            .normalize_or_zero();
        if n.length_squared() < 1e-6 || dir1.length_squared() < 1e-6 || dir2.length_squared() < 1e-6
        {
            continue;
        }

        let color = ctx.color(&ctx.style.translate, axis, GizmoOperation::TranslatePlane);

        let base = ctx.frame.origin
            + dir1 * ctx.style.translate_plane_offset
            + dir2 * ctx.style.translate_plane_offset;
        let p0 = base;
        let p1 = base + dir1 * ctx.style.translate_plane_size;
        let p2 =
            base + dir1 * ctx.style.translate_plane_size + dir2 * ctx.style.translate_plane_size;
        let p3 = base + dir2 * ctx.style.translate_plane_size;

        gizmos.line(p0, p1, color);
        gizmos.line(p1, p2, color);
        gizmos.line(p2, p3, color);
        gizmos.line(p3, p0, color);
    }
}

fn draw_scale_cubes(ctx: &GizmoDrawContext, gizmos: &mut Gizmos, axis_length: f32) {
    let half = ctx.style.scale_cube_size * 0.5;
    for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
        if !ctx.style.scale_axes.enabled(axis) {
            continue;
        }
        let axis_dir = ctx
            .frame
            .axis_dir(axis, AxisKind::Scale)
            .normalize_or_zero();
        if axis_dir.length_squared() < 1e-6 {
            continue;
        }

        let color = ctx.color(&ctx.style.scale, axis, GizmoOperation::ScaleAxis);

        let center = ctx.frame.origin + axis_dir * (axis_length * ctx.style.scale_cube_offset);

        let corners = [
            Vec3::new(-half, -half, -half),
            Vec3::new(-half, -half, half),
            Vec3::new(-half, half, -half),
            Vec3::new(-half, half, half),
            Vec3::new(half, -half, -half),
            Vec3::new(half, -half, half),
            Vec3::new(half, half, -half),
            Vec3::new(half, half, half),
        ];
        let corners: Vec<Vec3> = corners.iter().map(|c| center + *c).collect();

        let edges = [
            (0, 1),
            (0, 2),
            (0, 4),
            (1, 3),
            (1, 5),
            (2, 3),
            (2, 6),
            (3, 7),
            (4, 5),
            (4, 6),
            (5, 7),
            (6, 7),
        ];

        for (i0, i1) in edges {
            gizmos.line(corners[i0], corners[i1], color);
        }
    }
}

fn draw_rotation_arcs(ctx: &GizmoDrawContext, gizmos: &mut Gizmos, axis_length: f32) {
    let total_angle_radians = ctx.style.rotation_arc_degrees.to_radians();
    let radius = axis_length;
    let segments = ctx.style.rotation_arc_segments;

    for (axis, axis_vec, n1, n2) in [
        (
            GizmoAxis::X,
            ctx.frame.axis_dir(GizmoAxis::X, AxisKind::Rotate),
            ctx.frame.axis_dir(GizmoAxis::Y, AxisKind::Rotate),
            ctx.frame.axis_dir(GizmoAxis::Z, AxisKind::Rotate),
        ),
        (
            GizmoAxis::Y,
            ctx.frame.axis_dir(GizmoAxis::Y, AxisKind::Rotate),
            ctx.frame.axis_dir(GizmoAxis::Z, AxisKind::Rotate),
            ctx.frame.axis_dir(GizmoAxis::X, AxisKind::Rotate),
        ),
        (
            GizmoAxis::Z,
            ctx.frame.axis_dir(GizmoAxis::Z, AxisKind::Rotate),
            ctx.frame.axis_dir(GizmoAxis::X, AxisKind::Rotate),
            ctx.frame.axis_dir(GizmoAxis::Y, AxisKind::Rotate),
        ),
    ] {
        if !ctx.style.rotate_axes.enabled(axis) {
            continue;
        }
        draw_rotation_arc(
            gizmos,
            ctx.frame.origin,
            axis_vec,
            n1,
            n2,
            ctx.color(&ctx.style.rotate, axis, GizmoOperation::Rotate),
            radius,
            total_angle_radians,
            segments,
        );
    }
}

/// Draw the transform gizmo at the active target (if any).
pub fn draw_gizmo(
    state: Res<TransformGizmoState>,
    style: Res<TransformGizmoStyle>,
    targets: Query<(Entity, &GlobalTransform), With<TransformGizmoTarget>>,
    cameras: Query<(&Camera, &GlobalTransform), With<TransformGizmoCamera>>,
    mut gizmos: Gizmos,
) {
    let Some((_camera, camera_transform)) = cameras.iter().next() else {
        return;
    };

    for (entity, transform) in targets.iter() {
        let frame = GizmoFrame::new(transform, state.space);
        let axis_length = style.axis_length;

        let hover_axes: Vec<GizmoAxis> = if state.active_target == Some(entity) {
            if let (Some(axis), Some(op)) = (state.hovered_axis, state.hovered_op) {
                axes_involved(op, axis)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let active_axes: Vec<GizmoAxis> = if let Some(drag) = &state.drag {
            if drag.target == entity {
                axes_involved(drag.op, drag.axis)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let ctx = GizmoDrawContext {
            state: &state,
            style: &style,
            frame: &frame,
            target: entity,
            hover_axes,
            active_axes,
        };

        let show_translate = style.show_translate;
        let show_rotate = style.show_rotate;
        let show_scale = style.show_scale;

        if style.show_axis_lines {
            draw_axis_lines(&ctx, &mut gizmos, axis_length);
        }

        if show_translate {
            draw_translation_cones(&ctx, &mut gizmos, axis_length);
            if style.show_translate_planes {
                draw_translation_planes(&ctx, &mut gizmos);
            }
        }

        if show_scale {
            draw_scale_cubes(&ctx, &mut gizmos, axis_length);

            if style.show_scale_uniform {
                let colors = &style.scale_uniform_colors;
                let is_active = matches!(
                    state.drag.as_ref(),
                    Some(drag)
                        if drag.target == entity && matches!(drag.op, GizmoOperation::ScaleUniform)
                );
                let is_hovered = state.active_target == Some(entity)
                    && matches!(state.hovered_op, Some(GizmoOperation::ScaleUniform));

                let color = if is_active {
                    colors.active
                } else if is_hovered {
                    colors.hover
                } else {
                    colors.idle
                };

                draw_uniform_scale_square(
                    &mut gizmos,
                    frame.origin,
                    style.scale_uniform_size,
                    color,
                    camera_transform,
                );
            }
        }

        if show_rotate {
            draw_rotation_arcs(&ctx, &mut gizmos, axis_length);
        }

        if style.show_origin_dot {
            draw_origin_dot(
                &mut gizmos,
                frame.origin,
                style.origin_dot_size,
                style.origin_dot_color,
                camera_transform,
            );
        }
    }
}
