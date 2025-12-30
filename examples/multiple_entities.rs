//! Multiple entity selection with shared pivot example.
//!
//! Demonstrates manipulating multiple entities through a shared pivot point.
//! The gizmo controls the pivot, which in turn moves all selected entities.
//!
//! Controls:
//! - 1/2/3: Toggle selection of cubes
//! - A/D: Select all / Deselect all
//! - T/R/S: Toggle translate/rotate/scale handles
//! - Q: Toggle world/local space
//! - Z/X/C: Toggle translate snap X/Y/Z
//! - V: Toggle rotation snap
//! - B: Toggle scale snap

use bevy::prelude::*;
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

#[derive(Clone, Copy)]
struct PivotFrame {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

#[derive(Resource, Default)]
struct PivotHistory(Option<PivotFrame>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TransformGizmoPlugin)
        .init_resource::<Selection>()
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
    mut selection: ResMut<Selection>,
    mut commands: Commands,
    targets: Query<(Entity, &TargetIndex), With<Selectable>>,
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
        for (entity, ti) in &targets {
            if ti.0 == idx {
                if let Some(pos) = selection.0.iter().position(|e| *e == entity) {
                    selection.0.remove(pos);
                    commands.entity(entity).remove::<Selected>();
                } else {
                    selection.0.push(entity);
                    commands.entity(entity).insert(Selected);
                }
                break;
            }
        }
    }

    if keys.just_pressed(KeyCode::KeyA) {
        selection.0.clear();
        for (entity, _) in &targets {
            selection.0.push(entity);
            commands.entity(entity).insert(Selected);
        }
    }

    if keys.just_pressed(KeyCode::KeyD) {
        for entity in selection.0.drain(..) {
            commands.entity(entity).remove::<Selected>();
        }
    }
}

fn update_pivot(
    selection: Res<Selection>,
    state: Res<TransformGizmoState>,
    mut set: ParamSet<(
        Query<&mut Transform, With<GizmoPivot>>,
        Query<&Transform, With<Selectable>>,
    )>,
) {
    if state.drag.is_some() || selection.0.is_empty() {
        return;
    }

    let positions: Vec<Vec3> = {
        let q = set.p1();
        selection.0.iter().filter_map(|e| q.get(*e).ok()).map(|t| t.translation).collect()
    };

    if positions.is_empty() {
        return;
    }

    let center = positions.iter().sum::<Vec3>() / positions.len() as f32;

    if let Some(mut pivot) = set.p0().iter_mut().next() {
        pivot.translation = center;
    }
}

fn apply_pivot_delta(
    state: Res<TransformGizmoState>,
    selection: Res<Selection>,
    mut history: ResMut<PivotHistory>,
    mut set: ParamSet<(
        Query<&Transform, With<GizmoPivot>>,
        Query<&mut Transform, With<Selectable>>,
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

    let Some(last) = history.0 else {
        history.0 = Some(current);
        return;
    };

    let delta_r = current.rotation * last.rotation.conjugate();
    let delta_s = current.scale / last.scale;
    let old_p = last.translation;
    let new_p = current.translation;

    {
        let mut q = set.p1();
        for &entity in &selection.0 {
            if let Ok(mut t) = q.get_mut(entity) {
                let mut offset = t.translation - old_p;
                offset = Vec3::new(offset.x * delta_s.x, offset.y * delta_s.y, offset.z * delta_s.z);
                offset = delta_r * offset;
                t.translation = new_p + offset;
                t.rotation = delta_r * t.rotation;
                t.scale *= delta_s;
            }
        }
    }

    history.0 = Some(current);
}

fn update_hud(
    state: Res<TransformGizmoState>,
    style: Res<TransformGizmoStyle>,
    selection: Res<Selection>,
    snap: Res<TransformGizmoSnap>,
    mut query: Query<&mut Text, With<Hud>>,
) {
    let Ok(mut text) = query.single_mut() else { return };

    let on = |b: bool| if b { "on" } else { "off" };

    text.0 = format!(
        "Mode: {} | Space: {}\n\
         Show: T({}) R({}) S({})\n\
         Snap: T({}/{}/{}) R({}) S({})\n\
         Selected: {}\n\n\
         [1/2/3] toggle cubes [A/D] all/none\n\
         [T/R/S] toggle tools [Q] space\n\
         [Z/X/C] snap T [V] snap R [B] snap S",
        state.mode,
        state.space,
        on(style.show_translate),
        on(style.show_rotate),
        on(style.show_scale),
        on(snap.translate.x.is_some()),
        on(snap.translate.y.is_some()),
        on(snap.translate.z.is_some()),
        on(snap.rotate.x.is_some()),
        on(snap.scale.x.is_some()),
        selection.0.len(),
    );
}
