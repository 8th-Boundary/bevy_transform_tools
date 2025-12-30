//! Core types for the transform gizmo plugin.
//!
//! This module contains all the public types used to configure and interact
//! with the transform gizmo system.

use bevy::prelude::*;
use std::fmt;

/// Which transform component the gizmo is currently editing for UI purposes.
///
/// This is mostly useful for external UI to display the current mode.
/// Interaction logic uses [`GizmoOperation`] internally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformGizmoMode {
    /// Translation mode - move entities along axes or planes.
    #[default]
    Translate,
    /// Rotation mode - rotate entities around axes.
    Rotate,
    /// Scale mode - scale entities along axes or uniformly.
    Scale,
}

impl fmt::Display for TransformGizmoMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransformGizmoMode::Translate => f.write_str("Translate"),
            TransformGizmoMode::Rotate => f.write_str("Rotate"),
            TransformGizmoMode::Scale => f.write_str("Scale"),
        }
    }
}

/// Coordinate space used by the gizmo axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformGizmoSpace {
    /// Axes aligned to world coordinates (global X/Y/Z).
    World,
    /// Axes aligned to the target entity's local rotation.
    #[default]
    Local,
}

impl fmt::Display for TransformGizmoSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransformGizmoSpace::Local => f.write_str("Local"),
            TransformGizmoSpace::World => f.write_str("World"),
        }
    }
}

/// Marker component for cameras used by the transform gizmo.
///
/// Add this to any camera whose view should be used for gizmo interaction.
/// This lets you have multiple cameras in your app while keeping the gizmo
/// logic scoped to just the tagged ones.
///
/// # Example
///
/// ```ignore
/// commands.spawn((
///     Camera3d::default(),
///     Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
///     TransformGizmoCamera,
/// ));
/// ```
#[derive(Component)]
pub struct TransformGizmoCamera;

/// Marks an entity as controllable by the transform gizmo.
///
/// Entities with this component can be manipulated via gizmo handles.
/// Add [`GizmoActive`] to mark which target is currently selected.
///
/// # Example
///
/// ```ignore
/// commands.spawn((
///     Mesh3d(mesh),
///     MeshMaterial3d(material),
///     Transform::from_xyz(0.0, 1.0, 0.0),
///     TransformGizmoTarget,
///     GizmoActive,  // This entity is the active gizmo target
/// ));
/// ```
#[derive(Component)]
pub struct TransformGizmoTarget;

/// Marks a [`TransformGizmoTarget`] as the currently active/selected target.
///
/// The gizmo will be rendered on entities that have both `TransformGizmoTarget`
/// and `GizmoActive`. Only one entity should have this at a time; if multiple
/// exist, the first one found is used.
///
/// # Example
///
/// ```ignore
/// // Spawn an entity with the gizmo active
/// commands.spawn((
///     Mesh3d(mesh),
///     MeshMaterial3d(material),
///     Transform::from_xyz(0.0, 1.0, 0.0),
///     TransformGizmoTarget,
///     GizmoActive,
/// ));
///
/// // To switch selection, remove GizmoActive from one entity and add to another:
/// commands.entity(old_target).remove::<GizmoActive>();
/// commands.entity(new_target).insert(GizmoActive);
/// ```
#[derive(Component)]
pub struct GizmoActive;

/// Identifies which axis (X, Y, or Z) a gizmo handle operates on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoAxis {
    /// The X axis (typically red).
    X,
    /// The Y axis (typically green).
    Y,
    /// The Z axis (typically blue).
    Z,
}

impl GizmoAxis {
    /// Converts the axis to its corresponding unit vector.
    pub fn to_vec3(self) -> Vec3 {
        match self {
            GizmoAxis::X => Vec3::X,
            GizmoAxis::Y => Vec3::Y,
            GizmoAxis::Z => Vec3::Z,
        }
    }
}

/// The type of operation being performed by the gizmo.
///
/// This distinguishes between different manipulation modes like axis-constrained
/// translation vs planar translation, or per-axis scaling vs uniform scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoOperation {
    /// Translation constrained to a single axis.
    TranslateAxis,
    /// Translation constrained to a plane (two axes).
    TranslatePlane,
    /// Rotation around a single axis.
    Rotate,
    /// Scaling along a single axis.
    ScaleAxis,
    /// Uniform scaling on all axes simultaneously.
    ScaleUniform,
}

/// Information about an active drag operation.
///
/// This struct captures all the state needed to compute transform deltas
/// while the user is dragging a gizmo handle.
#[derive(Debug, Clone)]
pub struct TransformGizmoDrag {
    /// The entity being manipulated.
    pub target: Entity,
    /// The type of operation being performed.
    pub op: GizmoOperation,
    /// The primary axis involved in the operation.
    pub axis: GizmoAxis,
    /// The world-space origin of the gizmo when the drag started.
    pub origin: Vec3,
    /// The direction of the primary axis in world space.
    pub axis_dir: Vec3,
    /// Normal of the interaction plane used for mouse projection.
    pub plane_normal: Vec3,
    /// Origin point of the interaction plane.
    pub plane_origin: Vec3,
    /// First direction vector for planar operations.
    pub plane_dir1: Vec3,
    /// Second direction vector for planar operations.
    pub plane_dir2: Vec3,
    /// First axis for planar snapping.
    pub plane_axis1: GizmoAxis,
    /// Second axis for planar snapping.
    pub plane_axis2: GizmoAxis,
    /// The target's translation when the drag started.
    pub start_translation: Vec3,
    /// The target's rotation when the drag started.
    pub start_rotation: Quat,
    /// The target's scale when the drag started.
    pub start_scale: Vec3,
    /// Initial parameter value (distance or angle) at drag start.
    pub start_t: f32,
    /// Initial vector from origin to hit point (for planar/rotation ops).
    pub start_vector: Vec3,
}

/// Global state for the transform gizmo system.
///
/// This resource tracks the current mode, which entity is selected, what's
/// being hovered, and any active drag operation.
#[derive(Resource, Clone, Default)]
pub struct TransformGizmoState {
    /// Current editing mode (Translate/Rotate/Scale) for UI display.
    pub mode: TransformGizmoMode,
    /// Coordinate space for gizmo axes (World or Local).
    pub space: TransformGizmoSpace,
    /// The currently active target entity, if any.
    pub active_target: Option<Entity>,
    /// The axis currently being hovered, if any.
    pub hovered_axis: Option<GizmoAxis>,
    /// The operation type currently being hovered, if any.
    pub hovered_op: Option<GizmoOperation>,
    /// Active drag state while mouse button is held, if any.
    pub drag: Option<TransformGizmoDrag>,
}

/// Colors for a single gizmo element in different interaction states.
///
/// Each gizmo handle can have different colors for idle, hovered, and
/// actively dragged states to provide visual feedback.
#[derive(Clone, Debug)]
pub struct GizmoStateColors {
    /// Color when the element is not being interacted with.
    pub idle: Color,
    /// Color when the mouse is hovering over the element.
    pub hover: Color,
    /// Color when the element is being actively dragged.
    pub active: Color,
}

impl GizmoStateColors {
    /// Creates a new color set with the specified colors.
    pub fn new(idle: Color, hover: Color, active: Color) -> Self {
        Self {
            idle,
            hover,
            active,
        }
    }
}

impl Default for GizmoStateColors {
    fn default() -> Self {
        Self {
            idle: Color::srgb(0.8, 0.8, 0.8),
            hover: Color::srgb(1.0, 1.0, 1.0),
            active: Color::srgb(1.0, 1.0, 0.8),
        }
    }
}

/// Colors for each axis (X, Y, Z) of a gizmo handle group.
///
/// This allows customizing the appearance of translation, rotation, and
/// scale handles independently for each axis.
#[derive(Clone, Debug)]
pub struct AxisColors {
    /// Colors for the X axis (typically red tones).
    pub x: GizmoStateColors,
    /// Colors for the Y axis (typically green tones).
    pub y: GizmoStateColors,
    /// Colors for the Z axis (typically blue tones).
    pub z: GizmoStateColors,
}

impl AxisColors {
    /// Creates axis colors where all three axes use the same color set.
    pub fn uniform(colors: GizmoStateColors) -> Self {
        Self {
            x: colors.clone(),
            y: colors.clone(),
            z: colors,
        }
    }

    /// Creates axis colors with distinct colors for each axis.
    pub fn new(x: GizmoStateColors, y: GizmoStateColors, z: GizmoStateColors) -> Self {
        Self { x, y, z }
    }

    /// Returns the colors for a specific axis.
    pub fn for_axis(&self, axis: GizmoAxis) -> &GizmoStateColors {
        match axis {
            GizmoAxis::X => &self.x,
            GizmoAxis::Y => &self.y,
            GizmoAxis::Z => &self.z,
        }
    }
}

impl Default for AxisColors {
    fn default() -> Self {
        Self {
            x: GizmoStateColors::new(
                Color::srgb(1.0, 0.25, 0.25),
                Color::srgb(1.0, 0.7, 0.7),
                Color::srgb(1.0, 0.9, 0.9),
            ),
            y: GizmoStateColors::new(
                Color::srgb(0.25, 1.0, 0.25),
                Color::srgb(0.7, 1.0, 0.7),
                Color::srgb(0.9, 1.0, 0.9),
            ),
            z: GizmoStateColors::new(
                Color::srgb(0.25, 0.5, 1.0),
                Color::srgb(0.7, 0.8, 1.0),
                Color::srgb(0.9, 0.95, 1.0),
            ),
        }
    }
}

/// Per-axis enable/disable toggles for gizmo handles.
///
/// Use this to selectively show or hide individual axis handles.
#[derive(Clone, Copy, Debug, Default)]
pub struct AxisToggles {
    /// Whether the X axis handle is enabled.
    pub x: bool,
    /// Whether the Y axis handle is enabled.
    pub y: bool,
    /// Whether the Z axis handle is enabled.
    pub z: bool,
}

impl AxisToggles {
    /// Creates toggles with all axes enabled.
    pub fn all() -> Self {
        Self {
            x: true,
            y: true,
            z: true,
        }
    }

    /// Creates toggles with all axes disabled.
    pub fn none() -> Self {
        Self {
            x: false,
            y: false,
            z: false,
        }
    }

    /// Returns whether a specific axis is enabled.
    pub fn enabled(&self, axis: GizmoAxis) -> bool {
        match axis {
            GizmoAxis::X => self.x,
            GizmoAxis::Y => self.y,
            GizmoAxis::Z => self.z,
        }
    }
}

/// Optional per-axis snapping increments.
///
/// When set, transform operations will snap to multiples of the specified
/// values. Use `None` for an axis to disable snapping on that axis.
#[derive(Clone, Copy, Debug, Default)]
pub struct AxisSnap {
    /// Snap increment for the X axis, or `None` to disable.
    pub x: Option<f32>,
    /// Snap increment for the Y axis, or `None` to disable.
    pub y: Option<f32>,
    /// Snap increment for the Z axis, or `None` to disable.
    pub z: Option<f32>,
}

impl AxisSnap {
    /// Creates a snap configuration with no snapping on any axis.
    pub fn none() -> Self {
        Self {
            x: None,
            y: None,
            z: None,
        }
    }

    /// Creates a snap configuration with the same increment on all axes.
    pub fn uniform(increment: f32) -> Self {
        Self {
            x: Some(increment),
            y: Some(increment),
            z: Some(increment),
        }
    }

    /// Returns the snap increment for a specific axis.
    pub fn get(&self, axis: GizmoAxis) -> Option<f32> {
        match axis {
            GizmoAxis::X => self.x,
            GizmoAxis::Y => self.y,
            GizmoAxis::Z => self.z,
        }
    }
}

/// Snapping configuration for all transform operations.
///
/// This resource controls snap-to-grid behavior for translation, rotation,
/// and scaling operations.
#[derive(Resource, Clone, Default)]
pub struct TransformGizmoSnap {
    /// Snap increments for translation (in world units).
    pub translate: AxisSnap,
    /// Snap increments for rotation (in radians).
    pub rotate: AxisSnap,
    /// Snap increments for scale (as multipliers).
    pub scale: AxisSnap,
}

/// Visual style and sizing configuration for the transform gizmo.
///
/// This resource controls all aspects of gizmo appearance including colors,
/// sizes, and which elements are visible. Modify this at runtime to customize
/// the gizmo appearance.
#[derive(Resource, Clone)]
pub struct TransformGizmoStyle {
    // === Visibility toggles ===
    /// Whether to draw the primary XYZ axis lines.
    pub show_axis_lines: bool,
    /// Whether to show translation handles.
    pub show_translate: bool,
    /// Which axes have translation handles enabled.
    pub translate_axes: AxisToggles,
    /// Whether to show rotation handles.
    pub show_rotate: bool,
    /// Which axes have rotation handles enabled.
    pub rotate_axes: AxisToggles,
    /// Whether to show scale handles.
    pub show_scale: bool,
    /// Which axes have scale handles enabled.
    pub scale_axes: AxisToggles,

    // === General styling ===
    /// Line width for gizmo rendering (in pixels).
    pub line_width: f32,
    /// Depth bias to draw gizmos on top of regular geometry.
    /// Negative values bring the gizmo closer to the camera.
    pub depth_bias: f32,
    /// Length of each axis line (in world units).
    pub axis_length: f32,

    // === Colors ===
    /// Colors for the main axis lines.
    pub axis_lines: AxisColors,
    /// Colors for translation handles.
    pub translate: AxisColors,
    /// Colors for rotation handles.
    pub rotate: AxisColors,
    /// Colors for scale handles.
    pub scale: AxisColors,

    // === Translation cone handles ===
    /// Length of the translation cone from base to tip.
    pub translate_cone_length: f32,
    /// Radius of the translation cone at its base.
    pub translate_cone_radius: f32,
    /// Hit detection radius for translation cones.
    pub translate_hit_radius: f32,

    // === Scale cube handles ===
    /// Edge length of the scale cube handles.
    pub scale_cube_size: f32,
    /// Position of scale cubes as a fraction of axis_length.
    pub scale_cube_offset: f32,
    /// Hit detection radius for scale cubes.
    pub scale_hit_radius: f32,

    // === Rotation arc handles ===
    /// Angular extent of each rotation arc (in degrees).
    pub rotation_arc_degrees: f32,
    /// Number of line segments per rotation arc.
    pub rotation_arc_segments: usize,
    /// Visual thickness of rotation arcs.
    pub rotation_arc_thickness: f32,
    /// Hit detection thickness for rotation arcs.
    pub rotation_hit_thickness: f32,

    // === Hit detection ===
    /// Bounding sphere radius for early-out hit testing.
    pub bounds_radius: f32,

    // === Planar translation handles ===
    /// Whether to show planar translation handles (XY, XZ, YZ planes).
    pub show_translate_planes: bool,
    /// Side length of planar translation rectangles.
    pub translate_plane_size: f32,
    /// Offset of plane handles from the origin along each axis.
    pub translate_plane_offset: f32,
    /// Hit detection padding for planar handles.
    pub translate_plane_hit_thickness: f32,

    // === Uniform scale handle ===
    /// Whether to show the uniform scale handle at the origin.
    pub show_scale_uniform: bool,
    /// Side length of the uniform scale square.
    pub scale_uniform_size: f32,
    /// Hit detection radius for the uniform scale handle.
    pub scale_uniform_hit_radius: f32,
    /// Colors for the uniform scale handle.
    pub scale_uniform_colors: GizmoStateColors,

    // === Origin marker ===
    /// Whether to draw the origin marker.
    pub show_origin_dot: bool,
    /// Size of the origin marker.
    pub origin_dot_size: f32,
    /// Color of the origin marker.
    pub origin_dot_color: Color,
}

impl Default for TransformGizmoStyle {
    fn default() -> Self {
        let axis_colors = AxisColors::default();

        let axis_length = 2.0;
        let translate_cone_length = 0.4;
        let translate_cone_radius = 0.12;
        let scale_cube_size = 0.2;
        let scale_cube_offset = 0.7;

        let translate_hit_radius = translate_cone_length * 0.9;
        let scale_hit_radius = scale_cube_size * 0.9;
        let bounds_radius = axis_length + translate_cone_length + scale_cube_size;

        let translate_plane_size = 0.5;
        let translate_plane_offset = 0.35;

        let scale_uniform_colors = GizmoStateColors::new(
            Color::srgba(1.0, 1.0, 1.0, 0.9),
            Color::srgba(1.0, 1.0, 1.0, 1.0),
            Color::srgba(1.0, 0.9, 0.8, 1.0),
        );
        let scale_uniform_size = 0.27;
        let scale_uniform_hit_radius = 0.35;

        let origin_dot_size = 0.1;
        let origin_dot_color = Color::srgb(1.0, 0.6, 0.2);

        Self {
            show_axis_lines: true,
            show_translate: true,
            translate_axes: AxisToggles::all(),
            show_rotate: true,
            rotate_axes: AxisToggles::all(),
            show_scale: true,
            scale_axes: AxisToggles::all(),

            line_width: 4.0,
            depth_bias: -1.0,
            axis_length,

            axis_lines: axis_colors.clone(),
            translate: axis_colors.clone(),
            rotate: axis_colors.clone(),
            scale: axis_colors,

            translate_cone_length,
            translate_cone_radius,
            translate_hit_radius,

            scale_cube_size,
            scale_cube_offset,
            scale_hit_radius,

            rotation_arc_degrees: 30.0,
            rotation_arc_segments: 20,
            rotation_arc_thickness: 0.05,
            rotation_hit_thickness: 0.25,

            bounds_radius,

            show_translate_planes: true,
            translate_plane_size,
            translate_plane_offset,
            translate_plane_hit_thickness: 0.1,

            show_scale_uniform: true,
            scale_uniform_size,
            scale_uniform_hit_radius,
            scale_uniform_colors,

            show_origin_dot: true,
            origin_dot_size,
            origin_dot_color,
        }
    }
}
