//! Transform gizmo plugin for Bevy 0.17.x.
//!
//! This crate provides a 3D transform gizmo for manipulating entity transforms
//! in Bevy applications. It supports translation, rotation, and scaling with
//! both world and local coordinate spaces.
//!
//! # Quick Start
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_transform_tools::{
//!     TransformGizmoPlugin, TransformGizmoCamera, TransformGizmoTarget, GizmoActive,
//! };
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(TransformGizmoPlugin)
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     // Camera with gizmo support
//!     commands.spawn((
//!         Camera3d::default(),
//!         Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
//!         TransformGizmoCamera,
//!     ));
//!
//!     // Entity with active gizmo
//!     commands.spawn((
//!         Transform::from_xyz(0.0, 1.0, 0.0),
//!         TransformGizmoTarget,
//!         GizmoActive,  // This entity has the gizmo
//!     ));
//! }
//! ```
//!
//! # Features
//!
//! - **Translation**: Move entities along axes or planes (XY, XZ, YZ)
//! - **Rotation**: Rotate entities around any axis
//! - **Scaling**: Scale entities per-axis or uniformly
//! - **Coordinate Spaces**: World or local space manipulation
//! - **Snap-to-Grid**: Optional snapping for precise positioning
//! - **Customizable**: Full control over colors, sizes, and visibility
//!
//! # Configuration
//!
//! The gizmo can be configured through several resources:
//!
//! - [`TransformGizmoState`]: Current mode, selected target, and drag state
//! - [`TransformGizmoStyle`]: Visual appearance (colors, sizes, visibility)
//! - [`TransformGizmoSnap`]: Snap-to-grid increments for each operation

#![warn(missing_docs)]

use bevy::prelude::*;

mod draw;
mod gizmo_frame;
mod interaction;
mod math;
mod types;

// Re-export all public types
pub use types::{
    AxisColors, AxisSnap, AxisToggles, GizmoActive, GizmoAxis, GizmoOperation, GizmoStateColors,
    TransformGizmoCamera, TransformGizmoDrag, TransformGizmoMode, TransformGizmoSnap,
    TransformGizmoSpace, TransformGizmoState, TransformGizmoStyle, TransformGizmoTarget,
};

use crate::draw::draw_gizmo;
use crate::interaction::{begin_drag, configure_gizmos, drag_gizmo, end_drag, update_hovered_axis};

/// Syncs [`GizmoActive`] component with [`TransformGizmoState::active_target`].
///
/// This system finds entities with both `TransformGizmoTarget` and `GizmoActive`,
/// and sets the first one as the active target in the state resource.
fn sync_active_target(
    mut state: ResMut<TransformGizmoState>,
    query: Query<Entity, (With<TransformGizmoTarget>, With<GizmoActive>)>,
) {
    // Find the first entity with GizmoActive
    if let Some(entity) = query.iter().next() {
        state.active_target = Some(entity);
    }
}

/// Plugin that enables the transform gizmo system.
///
/// Add this plugin to your Bevy app to enable transform gizmo functionality.
/// The plugin registers the necessary resources and systems for gizmo
/// rendering and interaction.
///
/// # Example
///
/// ```ignore
/// use bevy::prelude::*;
/// use bevy_transform_tools::{TransformGizmoPlugin, TransformGizmoTarget, GizmoActive};
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(TransformGizmoPlugin)
///     .run();
/// ```
pub struct TransformGizmoPlugin;

impl Plugin for TransformGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransformGizmoState>()
            .init_resource::<TransformGizmoStyle>()
            .init_resource::<TransformGizmoSnap>()
            .add_systems(Startup, configure_gizmos)
            .add_systems(
                Update,
                (
                    sync_active_target,
                    update_hovered_axis,
                    begin_drag,
                    drag_gizmo,
                    end_drag,
                    draw_gizmo,
                )
                    .chain(),
            );
    }
}
