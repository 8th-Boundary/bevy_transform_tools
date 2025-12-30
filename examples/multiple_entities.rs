//! Multiple entity selection example.
//!
//! Demonstrates advanced usage with multiple selectable entities, a shared pivot,
//! and snap-to-grid functionality.
//!
//! Controls:
//! - 1/2/3: Toggle selection of individual cubes
//! - A: Select all, D: Clear selection
//! - T/R/S: Toggle translate/rotate/scale handles
//! - Q/Space: Toggle world/local space
//! - Tab: Toggle between editing pivot vs selection
//! - M/F/L/C: Set pivot mode (Manual/First/Last/Center)
//! - Z/X/C: Toggle translate snap on X/Y/Z
//! - V: Toggle rotation snap
//! - B: Toggle scale snap

use bevy::mesh::PlaneMeshBuilder;
use bevy::prelude::*;
use bevy_transform_tools::{
    TransformGizmoCamera, TransformGizmoMode, TransformGizmoPlugin, TransformGizmoSnap,
    TransformGizmoSpace, TransformGizmoState, TransformGizmoStyle, TransformGizmoTarget,
};

/// Index of a selectable cube, used for UI / input (1, 2, 3...).
#[derive(Component)]
struct TargetIndex(u8);

/// Marker for cubes that can be controlled by the gizmo.
#[derive(Component)]
struct Selectable;

/// Marker for cubes that are currently selected.
#[derive(Component)]
struct Selected;

/// Invisible (or tiny) pivot entity that the transform gizmo is attached to.
#[derive(Component)]
struct GizmoPivot;

#[derive(Component)]
struct GizmoHud;

/// How we compute the pivot position from the current selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PivotMode {
    /// Pivot does not automatically snap to selection; stays where the user puts it.
    Free,
    /// Pivot snaps to the first selected entity.
    First,
    /// Pivot snaps to the last selected entity.
    Last,
    /// Pivot snaps to the center of all selected entities.
    Center,
}

/// Whether we are editing the pivot itself, or applying changes to the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditMode {
    /// Move the pivot only (selection stays put).
    Pivot,
    /// Apply pivot motion as a delta to all selected entities.
    Selection,
}

/// Selection state and pivot/edit modes.
#[derive(Resource)]
struct MultiGizmoSelection {
    selected: Vec<Entity>,
    pivot_mode: PivotMode,
    edit_mode: EditMode,
}

impl Default for MultiGizmoSelection {
    fn default() -> Self {
        Self {
            selected: Vec::new(),
            pivot_mode: PivotMode::Free,
            edit_mode: EditMode::Selection,
        }
    }
}

/// Last known pivot transform, used to compute deltas while dragging.
#[derive(Clone, Copy)]
struct PivotFrame {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

#[derive(Resource, Default)]
struct PivotHistory {
    last: Option<PivotFrame>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TransformGizmoPlugin)
        .init_resource::<MultiGizmoSelection>()
        .init_resource::<PivotHistory>()
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(
            Update,
            (
                keyboard_gizmo_controls,
                selection_input,
                update_pivot_from_selection,
                apply_pivot_delta_to_selection,
                update_hud,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmo_state: ResMut<TransformGizmoState>,
) {
    // Ground plane.
    let plane_mesh = meshes.add(PlaneMeshBuilder::from_size(Vec2::splat(20.0)));
    let plane_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.18),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..Default::default()
    });

    commands.spawn((
        Mesh3d(plane_mesh),
        MeshMaterial3d(plane_mat),
        Transform::from_translation(Vec3::ZERO),
    ));

    // Simple lighting.
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 5000.0,
            ..Default::default()
        },
        Transform::from_xyz(-6.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Camera that drives the gizmo.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(8.0, 8.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        TransformGizmoCamera,
    ));

    // Some colorful cubes to manipulate.
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    let colors = [
        Color::srgb(0.9, 0.3, 0.3),
        Color::srgb(0.3, 0.9, 0.3),
        Color::srgb(0.3, 0.3, 0.9),
    ];

    let positions = [
        Vec3::new(-3.0, 0.75, 0.0),
        Vec3::new(0.0, 0.75, 0.0),
        Vec3::new(3.0, 0.75, 0.0),
    ];

    for (i, (color, position)) in colors.into_iter().zip(positions).enumerate() {
        let material = materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.6,
            metallic: 0.0,
            ..Default::default()
        });

        commands.spawn((
            Selectable,
            TargetIndex((i + 1) as u8),
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(position),
        ));
    }

    // Pivot entity controlled by the transform gizmo.
    let pivot = commands
        .spawn((
            GizmoPivot,
            TransformGizmoTarget,
            Transform::from_xyz(0.0, 0.75, 0.0),
        ))
        .id();

    // Attach gizmo to the pivot.
    gizmo_state.active_target = Some(pivot);
    gizmo_state.mode = TransformGizmoMode::Translate;
    gizmo_state.space = TransformGizmoSpace::Local;
}

/// Example keyboard wiring for gizmo mode / space.
fn keyboard_gizmo_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<TransformGizmoState>,
    mut style: ResMut<TransformGizmoStyle>,
    mut snap: ResMut<TransformGizmoSnap>,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        state.mode = TransformGizmoMode::Translate;
        style.show_translate = !style.show_translate;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        state.mode = TransformGizmoMode::Rotate;
        style.show_rotate = !style.show_rotate;
    }
    if keys.just_pressed(KeyCode::KeyS) {
        state.mode = TransformGizmoMode::Scale;
        style.show_scale = !style.show_scale;
    }
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::KeyQ) {
        state.space = match state.space {
            TransformGizmoSpace::World => TransformGizmoSpace::Local,
            TransformGizmoSpace::Local => TransformGizmoSpace::World,
        };
    }

    // Snap toggles:
    if keys.just_pressed(KeyCode::KeyZ) {
        // Toggle translate snapping on X axis at 0.5 units.
        snap.translate.x = match snap.translate.x {
            Some(_) => None,
            None => Some(0.5),
        };
    }
    if keys.just_pressed(KeyCode::KeyX) {
        // Toggle translate snapping on Y axis at 0.5 units.
        snap.translate.y = match snap.translate.y {
            Some(_) => None,
            None => Some(0.5),
        };
    }
    if keys.just_pressed(KeyCode::KeyC) {
        // Toggle translate snapping on Z axis at 0.5 units.
        snap.translate.z = match snap.translate.z {
            Some(_) => None,
            None => Some(0.5),
        };
    }

    if keys.just_pressed(KeyCode::KeyV) {
        // Toggle rotate snapping on all axes at 15 degrees (radians).
        let toggle = snap.rotate.x.is_none();
        let val = if toggle {
            Some(15f32.to_radians())
        } else {
            None
        };
        snap.rotate.x = val;
        snap.rotate.y = val;
        snap.rotate.z = val;
    }

    if keys.just_pressed(KeyCode::KeyB) {
        // Toggle scale snapping on all axes at 0.25 units.
        let toggle = snap.scale.x.is_none();
        let val = if toggle { Some(0.25) } else { None };
        snap.scale.x = val;
        snap.scale.y = val;
        snap.scale.z = val;
    }
}

/// Handle selection input and pivot/edit mode changes.
fn selection_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<MultiGizmoSelection>,
    mut commands: Commands,
    query_targets: Query<(Entity, &TargetIndex), With<Selectable>>,
) {
    // Toggle individual selection with 1/2/3.
    let mut toggle_index: Option<u8> = None;
    if keys.just_pressed(KeyCode::Digit1) {
        toggle_index = Some(1);
    } else if keys.just_pressed(KeyCode::Digit2) {
        toggle_index = Some(2);
    } else if keys.just_pressed(KeyCode::Digit3) {
        toggle_index = Some(3);
    }

    if let Some(idx) = toggle_index {
        for (entity, target_index) in &query_targets {
            if target_index.0 == idx {
                if let Some(pos) = selection.selected.iter().position(|e| *e == entity) {
                    selection.selected.remove(pos);
                    if let Ok(mut ec) = commands.get_entity(entity) {
                        ec.remove::<Selected>();
                    }
                } else {
                    selection.selected.push(entity);
                    if let Ok(mut ec) = commands.get_entity(entity) {
                        ec.insert(Selected);
                    }
                }
                break;
            }
        }
    }

    // A = select all.
    if keys.just_pressed(KeyCode::KeyA) {
        selection.selected.clear();
        for (entity, _) in &query_targets {
            selection.selected.push(entity);
            if let Ok(mut ec) = commands.get_entity(entity) {
                ec.insert(Selected);
            }
        }
    }

    // D = clear selection.
    if keys.just_pressed(KeyCode::KeyD) {
        for entity in selection.selected.drain(..) {
            if let Ok(mut ec) = commands.get_entity(entity) {
                ec.remove::<Selected>();
            }
        }
    }

    // Pivot mode:
    //   M = Free (do not snap to selection)
    //   F = First selected
    //   L = Last selected
    //   C = Center of all selected
    if keys.just_pressed(KeyCode::KeyM) {
        selection.pivot_mode = PivotMode::Free;
    }
    if keys.just_pressed(KeyCode::KeyF) {
        selection.pivot_mode = PivotMode::First;
    }
    if keys.just_pressed(KeyCode::KeyL) {
        selection.pivot_mode = PivotMode::Last;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        selection.pivot_mode = PivotMode::Center;
    }

    // Tab toggles between editing the pivot itself and applying changes to the selection.
    if keys.just_pressed(KeyCode::Tab) {
        selection.edit_mode = match selection.edit_mode {
            EditMode::Pivot => EditMode::Selection,
            EditMode::Selection => EditMode::Pivot,
        };
    }
}

/// Keep the pivot positioned relative to the current selection when in Selection edit mode.
///
/// - First: pivot snaps to the first selected entity.
/// - Last:  pivot snaps to the last selected entity.
/// - Center: pivot snaps to the average position of all selected entities.
fn update_pivot_from_selection(
    selection: Res<MultiGizmoSelection>,
    state: Res<TransformGizmoState>,
    mut queries: ParamSet<(
        Query<&mut Transform, With<GizmoPivot>>,
        Query<&Transform, With<Selectable>>,
    )>,
) {
    // Only auto-snap the pivot when we *aren't* currently dragging it, and only
    // when we are in EditMode::Selection.
    if state.drag.is_some() || selection.edit_mode != EditMode::Selection {
        return;
    }

    // In Free mode, never auto-snap the pivot to the selection.
    if matches!(selection.pivot_mode, PivotMode::Free) {
        return;
    }

    if selection.selected.is_empty() {
        return;
    }

    // First gather positions from the selectable targets (p1).
    let mut positions: Vec<Vec3> = Vec::new();
    {
        let targets = queries.p1();
        for entity in &selection.selected {
            if let Ok(t) = targets.get(*entity) {
                positions.push(t.translation);
            }
        }
    }

    if positions.is_empty() {
        return;
    }

    let new_pos = match selection.pivot_mode {
        PivotMode::First => positions.first().copied().unwrap(),
        PivotMode::Last => positions.last().copied().unwrap(),
        PivotMode::Center | PivotMode::Free => {
            let mut sum = Vec3::ZERO;
            for p in &positions {
                sum += *p;
            }
            sum / positions.len() as f32
        }
    };

    // Then move the pivot using the pivot query (p0).
    let mut pivot_query = queries.p0();
    let mut pivot = match pivot_query.iter_mut().next() {
        Some(p) => p,
        None => return,
    };

    pivot.translation = new_pos;
}

/// Apply the pivot's motion as a delta to all selected entities while dragging.
fn apply_pivot_delta_to_selection(
    state: Res<TransformGizmoState>,
    selection: Res<MultiGizmoSelection>,
    mut history: ResMut<PivotHistory>,
    mut queries: ParamSet<(
        Query<&Transform, With<GizmoPivot>>,
        Query<&mut Transform, With<Selectable>>,
    )>,
) {
    // Read the current pivot transform (p0).
    let current = {
        let pivot_query = queries.p0();
        let Some(pivot) = pivot_query.iter().next() else {
            return;
        };

        PivotFrame {
            translation: pivot.translation,
            rotation: pivot.rotation,
            scale: pivot.scale,
        }
    };

    // If we're not editing the selection or not dragging, just update history.
    if selection.edit_mode != EditMode::Selection || state.drag.is_none() {
        history.last = Some(current);
        return;
    }

    let Some(last) = history.last else {
        history.last = Some(current);
        return;
    };

    // Compute deltas between last and current pivot transform.
    let old_p = last.translation;
    let new_p = current.translation;

    let delta_r = current.rotation * last.rotation.conjugate();

    let safe_div = |a: Vec3, b: Vec3| {
        Vec3::new(
            if b.x.abs() < 1e-6 { 1.0 } else { a.x / b.x },
            if b.y.abs() < 1e-6 { 1.0 } else { a.y / b.y },
            if b.z.abs() < 1e-6 { 1.0 } else { a.z / b.z },
        )
    };
    let delta_s = safe_div(current.scale, last.scale);

    {
        let mut targets = queries.p1();
        for &entity in &selection.selected {
            if let Ok(mut transform) = targets.get_mut(entity) {
                // Position: offset from old pivot, then scale + rotate around pivot, then translate.
                let mut offset = transform.translation - old_p;
                offset = Vec3::new(
                    offset.x * delta_s.x,
                    offset.y * delta_s.y,
                    offset.z * delta_s.z,
                );
                offset = delta_r * offset;
                transform.translation = new_p + offset;

                // Orientation: apply the same delta rotation.
                transform.rotation = delta_r * transform.rotation;

                // Scale: apply delta scale.
                transform.scale = Vec3::new(
                    transform.scale.x * delta_s.x,
                    transform.scale.y * delta_s.y,
                    transform.scale.z * delta_s.z,
                );
            }
        }
    }

    history.last = Some(current);
}

fn setup_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..Default::default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            parent.spawn((Text::new(""), TextColor(Color::WHITE), GizmoHud));
        });
}

fn update_hud(
    state: Res<TransformGizmoState>,
    selection: Res<MultiGizmoSelection>,
    snap: Res<TransformGizmoSnap>,
    mut query: Query<&mut Text, With<GizmoHud>>,
) {
    let Some(mut text) = query.iter_mut().next() else {
        return;
    };

    let edit_str = match selection.edit_mode {
        EditMode::Pivot => "Pivot (move gizmo only)",
        EditMode::Selection => "Selection (apply to cubes)",
    };

    let pivot_str = match selection.pivot_mode {
        PivotMode::Free => "Free (manual)",
        PivotMode::First => "First",
        PivotMode::Last => "Last",
        PivotMode::Center => "Center",
    };

    let selected_count = selection.selected.len();

    let t_snap = (
        snap.translate.x.is_some(),
        snap.translate.y.is_some(),
        snap.translate.z.is_some(),
    );
    let r_snap = snap.rotate.x.is_some();
    let s_snap = snap.scale.x.is_some();

    let value = format!(
        "Transform Gizmo (multi-entity)\nMode:   {mode}\nSpace:  {space}\nEdit:   {edit}\nPivot:  {pivot}\nSelected: {count} cube(s)\n\nControls:\nT/R/S - Toggle show Translate/Rotate/Scale handles\nQ or Space - Toggle World/Local space\n1/2/3 - Toggle select cubes\nA - Select all, D - Clear selection\nM - Pivot Free (manual, do not snap)\nF/L/C - Pivot First / Last / Center\nTab - Toggle Edit Pivot vs Edit Selection\nLMB drag axis/ring to move/rotate/scale\n\nSnapping:\nZ/X/C - Toggle translate snap (X/Y/Z @ 0.5)\nV     - Toggle rotate snap (all axes @ 15 deg)\nB     - Toggle scale snap (all axes @ 0.25)\nCurrent snap: T({tx}/{ty}/{tz}) R({r}) S({s})",
        mode = state.mode,
        space = state.space,
        edit = edit_str,
        pivot = pivot_str,
        count = selected_count,
        tx = if t_snap.0 { "on" } else { "off" },
        ty = if t_snap.1 { "on" } else { "off" },
        tz = if t_snap.2 { "on" } else { "off" },
        r = if r_snap { "on" } else { "off" },
        s = if s_snap { "on" } else { "off" },
    );

    text.0 = value;
}
