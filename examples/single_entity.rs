//! Single entity gizmo example.
//!
//! Demonstrates the basic usage of the transform gizmo with a single entity.
//! Use T/R/S to toggle handles (and set the active tool), Q to toggle coordinate space.

use bevy::prelude::*;
use bevy_transform_tools::{
    GizmoActive, TransformGizmoCamera, TransformGizmoMode, TransformGizmoPlugin,
    TransformGizmoSpace, TransformGizmoState, TransformGizmoStyle, TransformGizmoTarget,
};

#[derive(Component)]
struct Hud;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TransformGizmoPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (keyboard_controls, update_hud))
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
        Transform::from_xyz(6.0, 6.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
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
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0)))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.35, 0.18))),
    ));

    // Cube with gizmo
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_length(1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.7, 1.0))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        TransformGizmoTarget,
        GizmoActive,
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
    if keys.just_pressed(KeyCode::KeyQ) {
        state.space = match state.space {
            TransformGizmoSpace::World => TransformGizmoSpace::Local,
            TransformGizmoSpace::Local => TransformGizmoSpace::World,
        };
    }
}

fn update_hud(
    state: Res<TransformGizmoState>,
    style: Res<TransformGizmoStyle>,
    mut query: Query<&mut Text, With<Hud>>,
) {
    let Ok(mut text) = query.single_mut() else { return };

    let on = |b: bool| if b { "on" } else { "off" };

    text.0 = format!(
        "Space: {}\n\
         Handles: T({}) R({}) S({})\n\n\
         [T/R/S] toggle handles (set tool)\n\
         [Q] toggle world/local",
        state.space,
        on(style.show_translate),
        on(style.show_rotate),
        on(style.show_scale),
    );
}
