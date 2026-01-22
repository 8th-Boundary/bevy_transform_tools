//! Multiple entity selection with shared pivot example.
//!
//! Demonstrates manipulating multiple entities through a shared pivot point.
//! The gizmo controls the pivot, which in turn moves all selected entities.
//!
//! Controls:
//! - 1/2/3: Toggle selection of cubes
//! - A/D: Select all / Deselect all
//! - T/R/S: Toggle translate/rotate/scale handles (also sets tool)
//! - Q: Toggle world/local space
//! - P: Cycle pivot mode (First/Last/Centroid/Keep Offset)
//! - Z/X/C: Toggle translate snap X/Y/Z
//! - V: Toggle rotation snap
//! - B: Toggle scale snap

use bevy::prelude::*;
use std::{collections::HashMap, fmt};
use bevy_transform_tools::{
    GizmoActive, TransformGizmoCamera, TransformGizmoMode, TransformGizmoPlugin,
    TransformGizmoSnap, TransformGizmoSpace, TransformGizmoState, TransformGizmoStyle,
    TransformGizmoTarget,
};

#[derive(Component)]
struct TargetIndex(u8);

#[derive(Component)]
struct Selectable;

#[derive(Component)]
struct Selected;

#[derive(Component)]
struct GizmoPivot;

#[derive(Component)]
struct Hud;

#[derive(Resource, Default)]
struct Selection(Vec<Entity>);

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum PivotMode {
    First,
    Last,
    Centroid,
    KeepOffset,
}

impl Default for PivotMode {
    fn default() -> Self {
        PivotMode::Centroid
    }
}

impl PivotMode {
    fn next(self) -> Self {
        match self {
            PivotMode::First => PivotMode::Last,
            PivotMode::Last => PivotMode::Centroid,
            PivotMode::Centroid => PivotMode::KeepOffset,
            PivotMode::KeepOffset => PivotMode::First,
        }
    }
}

impl fmt::Display for PivotMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PivotMode::First => f.write_str("First"),
            PivotMode::Last => f.write_str("Last"),
            PivotMode::Centroid => f.write_str("Centroid"),
            PivotMode::KeepOffset => f.write_str("Keep Offset"),
        }
    }
}

#[derive(Clone, Copy)]
struct PivotFrame {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

#[derive(Resource, Default)]
struct PivotHistory(Option<PivotFrame>);

#[derive(Clone, Copy)]
struct PivotOffset {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

#[derive(Resource, Default)]
struct PivotOffsets(HashMap<Entity, PivotOffset>);

const SCALE_EPS: f32 = 1e-6;

fn safe_div_vec3(value: Vec3, denom: Vec3) -> Vec3 {
    Vec3::new(
        if denom.x.abs() > SCALE_EPS { value.x / denom.x } else { 0.0 },
        if denom.y.abs() > SCALE_EPS { value.y / denom.y } else { 0.0 },
        if denom.z.abs() > SCALE_EPS { value.z / denom.z } else { 0.0 },
    )
}

fn capture_offset(pivot: &Transform, target: &Transform) -> PivotOffset {
    let inv_rot = pivot.rotation.conjugate();
    let delta = target.translation - pivot.translation;
    let unrotated = inv_rot * delta;

    let translation = safe_div_vec3(unrotated, pivot.scale);
    let rotation = inv_rot * target.rotation;
    let scale = safe_div_vec3(target.scale, pivot.scale);

    PivotOffset {
        translation,
        rotation,
        scale,
    }
}

fn apply_offset(pivot: &Transform, offset: &PivotOffset, target: &mut Transform) {
    let scaled_offset = offset.translation * pivot.scale;
    let rotated_offset = pivot.rotation * scaled_offset;

    target.translation = pivot.translation + rotated_offset;
    target.rotation = pivot.rotation * offset.rotation;
    target.scale = pivot.scale * offset.scale;
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TransformGizmoPlugin)
        .init_resource::<Selection>()
        .init_resource::<PivotMode>()
        .init_resource::<PivotOffsets>()
        .init_resource::<PivotHistory>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (keyboard_controls, selection_input, update_pivot, apply_pivot_delta, update_hud),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(8.0, 8.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        TransformGizmoCamera,
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 5000.0,
            ..default()
        },
        Transform::from_xyz(-6.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0)))),
        MeshMaterial3d(materials.add(Color::srgb(0.15, 0.15, 0.18))),
    ));

    // Cubes
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let colors = [
        Color::srgb(0.9, 0.3, 0.3),
        Color::srgb(0.3, 0.9, 0.3),
        Color::srgb(0.3, 0.3, 0.9),
    ];
    let positions = [
        Vec3::new(-3.0, 0.5, 0.0),
        Vec3::new(0.0, 0.5, 0.0),
        Vec3::new(3.0, 0.5, 0.0),
    ];

    for (i, (color, pos)) in colors.into_iter().zip(positions).enumerate() {
        commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(materials.add(color)),
            Transform::from_translation(pos),
            Selectable,
            TargetIndex((i + 1) as u8),
        ));
    }

    // Pivot entity
    commands.spawn((
        GizmoPivot,
        TransformGizmoTarget,
        GizmoActive,
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // HUD
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
    )).with_children(|p| {
        p.spawn((
            Text::new(""),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::WHITE),
            Hud,
        ));
    });
}

fn keyboard_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<TransformGizmoState>,
    mut style: ResMut<TransformGizmoStyle>,
    mut snap: ResMut<TransformGizmoSnap>,
    mut pivot_mode: ResMut<PivotMode>,
    selection: Res<Selection>,
    mut offsets: ResMut<PivotOffsets>,
    pivot_query: Query<&Transform, With<GizmoPivot>>,
    targets: Query<(Entity, &Transform), With<Selectable>>,
) {
    // Mode + toggle visibility
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
    if keys.just_pressed(KeyCode::KeyQ) {
        state.space = match state.space {
            TransformGizmoSpace::World => TransformGizmoSpace::Local,
            TransformGizmoSpace::Local => TransformGizmoSpace::World,
        };
    }
    if keys.just_pressed(KeyCode::KeyP) {
        *pivot_mode = pivot_mode.next();
        if matches!(*pivot_mode, PivotMode::KeepOffset) {
            offsets.0.clear();
            if let Some(pivot) = pivot_query.iter().next() {
                for &entity in &selection.0 {
                    if let Ok((_, transform)) = targets.get(entity) {
                        offsets.0.insert(entity, capture_offset(pivot, transform));
                    }
                }
            }
        } else {
            offsets.0.clear();
        }
    }

    // Translate snap
    if keys.just_pressed(KeyCode::KeyZ) {
        snap.translate.x = snap.translate.x.map_or(Some(0.5), |_| None);
    }
    if keys.just_pressed(KeyCode::KeyX) {
        snap.translate.y = snap.translate.y.map_or(Some(0.5), |_| None);
    }
    if keys.just_pressed(KeyCode::KeyC) {
        snap.translate.z = snap.translate.z.map_or(Some(0.5), |_| None);
    }

    // Rotate snap (15 degrees)
    if keys.just_pressed(KeyCode::KeyV) {
        let val = snap.rotate.x.map_or(Some(15f32.to_radians()), |_| None);
        snap.rotate.x = val;
        snap.rotate.y = val;
        snap.rotate.z = val;
    }

    // Scale snap
    if keys.just_pressed(KeyCode::KeyB) {
        let val = snap.scale.x.map_or(Some(0.25), |_| None);
        snap.scale.x = val;
        snap.scale.y = val;
        snap.scale.z = val;
    }
}

fn selection_input(
    keys: Res<ButtonInput<KeyCode>>,
    pivot_mode: Res<PivotMode>,
    mut selection: ResMut<Selection>,
    mut offsets: ResMut<PivotOffsets>,
    mut commands: Commands,
    pivot_query: Query<&Transform, With<GizmoPivot>>,
    targets: Query<(Entity, &TargetIndex, &Transform), With<Selectable>>,
) {
    let index = if keys.just_pressed(KeyCode::Digit1) {
        Some(1)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(2)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(3)
    } else {
        None
    };

    if let Some(idx) = index {
        for (entity, ti, transform) in &targets {
            if ti.0 == idx {
                if let Some(pos) = selection.0.iter().position(|e| *e == entity) {
                    selection.0.remove(pos);
                    commands.entity(entity).remove::<Selected>();
                    if matches!(*pivot_mode, PivotMode::KeepOffset) {
                        offsets.0.remove(&entity);
                    }
                } else {
                    selection.0.push(entity);
                    commands.entity(entity).insert(Selected);
                    if matches!(*pivot_mode, PivotMode::KeepOffset) {
                        if let Some(pivot) = pivot_query.iter().next() {
                            offsets.0.insert(entity, capture_offset(pivot, transform));
                        }
                    }
                }
                break;
            }
        }
    }

    if keys.just_pressed(KeyCode::KeyA) {
        selection.0.clear();
        let mut all: Vec<(u8, Entity, &Transform)> =
            targets.iter().map(|(e, ti, t)| (ti.0, e, t)).collect();
        all.sort_by_key(|(idx, _, _)| *idx);
        let keep_offsets = matches!(*pivot_mode, PivotMode::KeepOffset);
        let pivot = if keep_offsets {
            offsets.0.clear();
            pivot_query.iter().next()
        } else {
            None
        };
        for (_, entity, transform) in all {
            selection.0.push(entity);
            commands.entity(entity).insert(Selected);
            if let (true, Some(pivot)) = (keep_offsets, pivot) {
                offsets.0.insert(entity, capture_offset(pivot, transform));
            }
        }
    }

    if keys.just_pressed(KeyCode::KeyD) {
        for entity in selection.0.drain(..) {
            commands.entity(entity).remove::<Selected>();
        }
        if matches!(*pivot_mode, PivotMode::KeepOffset) {
            offsets.0.clear();
        }
    }
}

fn update_pivot(
    selection: Res<Selection>,
    state: Res<TransformGizmoState>,
    pivot_mode: Res<PivotMode>,
    mut set: ParamSet<(
        Query<&mut Transform, With<GizmoPivot>>,
        Query<&Transform, With<Selectable>>,
    )>,
) {
    if state.drag.is_some() || selection.0.is_empty() {
        return;
    }

    if matches!(*pivot_mode, PivotMode::KeepOffset) {
        return;
    }

    let target = {
        let q = set.p1();
        match *pivot_mode {
            PivotMode::First => selection
                .0
                .iter()
                .find_map(|e| q.get(*e).ok().map(|t| t.translation)),
            PivotMode::Last => selection
                .0
                .iter()
                .rev()
                .find_map(|e| q.get(*e).ok().map(|t| t.translation)),
            PivotMode::Centroid => {
                let mut sum = Vec3::ZERO;
                let mut count = 0usize;
                for entity in &selection.0 {
                    if let Ok(t) = q.get(*entity) {
                        sum += t.translation;
                        count += 1;
                    }
                }
                if count > 0 {
                    Some(sum / count as f32)
                } else {
                    None
                }
            }
            PivotMode::KeepOffset => None,
        }
    };

    if let (Some(pos), Some(mut pivot)) = (target, set.p0().iter_mut().next()) {
        pivot.translation = pos;
    }
}

fn apply_pivot_delta(
    state: Res<TransformGizmoState>,
    selection: Res<Selection>,
    pivot_mode: Res<PivotMode>,
    mut offsets: ResMut<PivotOffsets>,
    mut history: ResMut<PivotHistory>,
    mut set: ParamSet<(
        Query<&Transform, With<GizmoPivot>>,
        Query<(Entity, &mut Transform), With<Selectable>>,
    )>,
) {
    let current = {
        let q = set.p0();
        let Some(pivot) = q.iter().next() else { return };
        PivotFrame {
            translation: pivot.translation,
            rotation: pivot.rotation,
            scale: pivot.scale,
        }
    };

    if state.drag.is_none() {
        history.0 = Some(current);
        return;
    }

    if matches!(*pivot_mode, PivotMode::KeepOffset) {
        let pivot = Transform {
            translation: current.translation,
            rotation: current.rotation,
            scale: current.scale,
        };
        let mut q = set.p1();
        for &entity in &selection.0 {
            if let Ok((_entity, mut t)) = q.get_mut(entity) {
                let offset = offsets.0.get(&entity).copied().unwrap_or_else(|| {
                    let captured = capture_offset(&pivot, &t);
                    offsets.0.insert(entity, captured);
                    captured
                });
                apply_offset(&pivot, &offset, &mut t);
            }
        }
        history.0 = Some(current);
        return;
    }

    let Some(last) = history.0 else {
        history.0 = Some(current);
        return;
    };

    let delta_r = current.rotation * last.rotation.conjugate();
    let delta_s = current.scale / last.scale;
    let old_p = last.translation;
    let new_p = current.translation;

    let mut q = set.p1();
    for &entity in &selection.0 {
        if let Ok((_entity, mut t)) = q.get_mut(entity) {
            let mut offset = t.translation - old_p;
            offset = Vec3::new(offset.x * delta_s.x, offset.y * delta_s.y, offset.z * delta_s.z);
            offset = delta_r * offset;
            t.translation = new_p + offset;
            t.rotation = delta_r * t.rotation;
            t.scale *= delta_s;
        }
    }

    history.0 = Some(current);
}

fn update_hud(
    state: Res<TransformGizmoState>,
    style: Res<TransformGizmoStyle>,
    selection: Res<Selection>,
    snap: Res<TransformGizmoSnap>,
    pivot_mode: Res<PivotMode>,
    mut query: Query<&mut Text, With<Hud>>,
) {
    let Ok(mut text) = query.single_mut() else { return };

    let on = |b: bool| if b { "on" } else { "off" };

    text.0 = format!(
        "Tool: {} | Space: {}\n\
         Handles: T({}) R({}) S({})\n\
         Pivot: {} | Snap: T({}/{}/{}) R({}) S({})\n\
         Selected: {}\n\n\
         [1/2/3] toggle cubes  [A] all  [D] none\n\
         [T/R/S] toggle handles (set tool)\n\
         [Q] toggle world/local\n\
         [P] pivot mode\n\
         [Z/X/C] snap translate  [V] snap rotate  [B] snap scale",
        state.mode,
        state.space,
        on(style.show_translate),
        on(style.show_rotate),
        on(style.show_scale),
        *pivot_mode,
        on(snap.translate.x.is_some()),
        on(snap.translate.y.is_some()),
        on(snap.translate.z.is_some()),
        on(snap.rotate.x.is_some()),
        on(snap.scale.x.is_some()),
        selection.0.len(),
    );
}
