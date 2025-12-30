//! Single entity gizmo example.
//!
//! Demonstrates the basic usage of the transform gizmo with a single entity.
//! Use T/R/S keys to switch modes, Q to toggle coordinate space.

use bevy::mesh::PlaneMeshBuilder;
use bevy::prelude::*;
use bevy_transform_tools::{
    TransformGizmoCamera, TransformGizmoMode, TransformGizmoPlugin, TransformGizmoSpace,
    TransformGizmoState, TransformGizmoTarget,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TransformGizmoPlugin)
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(Update, (keyboard_gizmo_controls, update_hud))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmo_state: ResMut<TransformGizmoState>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        TransformGizmoCamera,
        Transform::from_xyz(6.0, 6.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::from_xyz(10.0, 15.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground plane
    let ground_mesh = meshes.add(PlaneMeshBuilder::from_size(Vec2::splat(20.0)));
    let ground_material = materials.add(Color::srgb(0.2, 0.35, 0.18));
    commands.spawn((
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Cube to manipulate
    let cube_mesh = meshes.add(Cuboid::from_length(1.0));
    let cube_material = materials.add(Color::srgb(0.2, 0.7, 1.0));

    let cube = commands
        .spawn((
            TransformGizmoTarget,
            Mesh3d(cube_mesh),
            MeshMaterial3d(cube_material),
            Transform::from_xyz(0.0, 0.5, 0.0),
        ))
        .id();

    // Tell the gizmo to use this cube as the active target.
    gizmo_state.active_target = Some(cube);
    gizmo_state.mode = TransformGizmoMode::Translate;
}

#[derive(Component)]
struct GizmoHud;

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

fn keyboard_gizmo_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<TransformGizmoState>,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        state.mode = TransformGizmoMode::Translate;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        state.mode = TransformGizmoMode::Rotate;
    }
    if keys.just_pressed(KeyCode::KeyS) {
        state.mode = TransformGizmoMode::Scale;
    }
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::KeyQ) {
        state.space = match state.space {
            TransformGizmoSpace::World => TransformGizmoSpace::Local,
            TransformGizmoSpace::Local => TransformGizmoSpace::World,
        };
    }
}

fn update_hud(state: Res<TransformGizmoState>, mut query: Query<&mut Text, With<GizmoHud>>) {
    let Some(mut text) = query.iter_mut().next() else {
        return;
    };

    let value = format!(
        "Transform Gizmo\nMode:  {mode}\nSpace: {space}\n\nControls:\nT - Translate\nR - Rotate\nS - Scale\nQ - Toggle World/Local\nLMB drag axis/ring to manipulate",
        mode = state.mode,
        space = state.space,
    );

    text.0 = value;
}
