//! Multiple gizmo targets example.
//!
//! Demonstrates switching between multiple entities with the gizmo.
//! Use 1/2/3 to select cubes, T/R/S to switch modes, Q to toggle space.

use bevy::prelude::*;
use bevy_transform_tools::{
    GizmoActive, TransformGizmoCamera, TransformGizmoMode, TransformGizmoPlugin,
    TransformGizmoSpace, TransformGizmoState, TransformGizmoTarget,
};

#[derive(Component)]
struct TargetIndex(u8);

#[derive(Component)]
struct Hud;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TransformGizmoPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (keyboard_controls, select_target, update_hud))
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
        Transform::from_xyz(8.0, 6.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        TransformGizmoCamera,
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 15.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(15.0)))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.35, 0.18))),
    ));

    // Three cubes - first one starts active
    let cube = meshes.add(Cuboid::from_length(1.5));
    let colors = [
        Color::srgb(0.9, 0.3, 0.3),
        Color::srgb(0.3, 0.9, 0.3),
        Color::srgb(0.3, 0.3, 0.9),
    ];
    let positions = [
        Vec3::new(-4.0, 0.75, 0.0),
        Vec3::new(0.0, 0.75, 0.0),
        Vec3::new(4.0, 0.75, 0.0),
    ];

    for (i, (color, pos)) in colors.into_iter().zip(positions).enumerate() {
        let mut entity = commands.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(materials.add(color)),
            Transform::from_translation(pos),
            TransformGizmoTarget,
            TargetIndex((i + 1) as u8),
        ));
        if i == 0 {
            entity.insert(GizmoActive);
        }
    }

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

fn keyboard_controls(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<TransformGizmoState>) {
    if keys.just_pressed(KeyCode::KeyT) {
        state.mode = TransformGizmoMode::Translate;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        state.mode = TransformGizmoMode::Rotate;
    }
    if keys.just_pressed(KeyCode::KeyS) {
        state.mode = TransformGizmoMode::Scale;
    }
    if keys.just_pressed(KeyCode::KeyQ) {
        state.space = match state.space {
            TransformGizmoSpace::World => TransformGizmoSpace::Local,
            TransformGizmoSpace::Local => TransformGizmoSpace::World,
        };
    }
}

/// Switch active target with 1/2/3 keys.
fn select_target(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    targets: Query<(Entity, &TargetIndex), With<TransformGizmoTarget>>,
    active: Query<Entity, With<GizmoActive>>,
) {
    let index = if keys.just_pressed(KeyCode::Digit1) {
        1
    } else if keys.just_pressed(KeyCode::Digit2) {
        2
    } else if keys.just_pressed(KeyCode::Digit3) {
        3
    } else {
        return;
    };

    // Remove GizmoActive from current
    for entity in &active {
        commands.entity(entity).remove::<GizmoActive>();
    }

    // Add GizmoActive to selected
    for (entity, target_index) in &targets {
        if target_index.0 == index {
            commands.entity(entity).insert(GizmoActive);
            break;
        }
    }
}

fn update_hud(
    state: Res<TransformGizmoState>,
    active: Query<&TargetIndex, With<GizmoActive>>,
    mut query: Query<&mut Text, With<Hud>>,
) {
    let Ok(mut text) = query.single_mut() else { return };

    let selected = active.iter().next().map_or(0, |t| t.0);

    text.0 = format!(
        "Mode: {} | Space: {}\nSelected: Cube {}\n\n\
         [1/2/3] Select cube\n\
         [T] Translate [R] Rotate [S] Scale\n\
         [Q] Toggle World/Local",
        state.mode,
        state.space,
        selected,
    );
}
