//! Gizmo coordinate frame handling.
//!
//! This module provides utilities for computing the coordinate frame
//! (origin and axis directions) for a gizmo based on the target entity's
//! transform and the selected coordinate space.

use bevy::prelude::*;

use crate::types::{GizmoAxis, TransformGizmoSpace};

/// Which flavor of axes to request from a gizmo frame.
pub enum AxisKind {
    Translate,
    Rotate,
    Scale,
}

/// Precomputed basis vectors for a gizmo target, respecting world/local space.
#[derive(Clone, Copy)]
pub struct GizmoFrame {
    pub origin: Vec3,
    tx_x: Vec3,
    tx_y: Vec3,
    tx_z: Vec3,
    sc_x: Vec3,
    sc_y: Vec3,
    sc_z: Vec3,
}

impl GizmoFrame {
    pub fn new(transform: &GlobalTransform, space: TransformGizmoSpace) -> Self {
        let origin = transform.translation();
        let rotation = transform.rotation();
        let local_x = rotation * Vec3::X;
        let local_y = rotation * Vec3::Y;
        let local_z = rotation * Vec3::Z;

        // Translation / rotation may be world or local.
        let (tx_x, tx_y, tx_z) = match space {
            TransformGizmoSpace::World => (Vec3::X, Vec3::Y, Vec3::Z),
            TransformGizmoSpace::Local => (local_x, local_y, local_z),
        };

        // Scale is always local to avoid surprising behaviour.
        let (sc_x, sc_y, sc_z) = (local_x, local_y, local_z);

        Self {
            origin,
            tx_x,
            tx_y,
            tx_z,
            sc_x,
            sc_y,
            sc_z,
        }
    }

    pub fn axis_dir(&self, axis: GizmoAxis, kind: AxisKind) -> Vec3 {
        match kind {
            AxisKind::Translate | AxisKind::Rotate => match axis {
                GizmoAxis::X => self.tx_x,
                GizmoAxis::Y => self.tx_y,
                GizmoAxis::Z => self.tx_z,
            },
            AxisKind::Scale => match axis {
                GizmoAxis::X => self.sc_x,
                GizmoAxis::Y => self.sc_y,
                GizmoAxis::Z => self.sc_z,
            },
        }
    }
}

/// Axes that bound the plane whose normal is `normal_axis`.
pub fn plane_axes(normal_axis: GizmoAxis) -> (GizmoAxis, GizmoAxis) {
    match normal_axis {
        GizmoAxis::X => (GizmoAxis::Y, GizmoAxis::Z),
        GizmoAxis::Y => (GizmoAxis::X, GizmoAxis::Z),
        GizmoAxis::Z => (GizmoAxis::X, GizmoAxis::Y),
    }
}
