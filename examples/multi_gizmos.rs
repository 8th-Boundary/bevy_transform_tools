use bevy::mesh::PlaneMeshBuilder;
use bevy::prelude::*;
use bevy_transform_tools::{
    TransformGizmoCamera, TransformGizmoMode, TransformGizmoPlugin, TransformGizmoSpace,
    TransformGizmoState, TransformGizmoTarget,
};

#[derive(Component)]
struct TargetIndex(u8);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TransformGizmoPlugin)
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(Update, (keyboard_gizmo_controls, select_target, update_hud))
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
        Transform::from_xyz(8.0, 6.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::from_xyz(10.0, 15.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground
    let ground_mesh = meshes.add(PlaneMeshBuilder::from_size(Vec2::splat(30.0)));
    let ground_material = materials.add(Color::srgb(0.2, 0.35, 0.18));
    commands.spawn((
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let cube_mesh = meshes.add(Cuboid::from_length(1.5));

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

    let mut first_entity = None;

    for (i, (color, position)) in colors.into_iter().zip(positions).enumerate() {
        let material = materials.add(color);
        let entity = commands
            .spawn((
                TransformGizmoTarget,
                TargetIndex((i + 1) as u8),
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(position),
            ))
            .id();

        if i == 0 {
            first_entity = Some(entity);
        }
    }

    gizmo_state.active_target = first_entity;
    gizmo_state.mode = TransformGizmoMode::Translate;
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

/// Select which cube is controlled by the gizmo.
/// 1, 2, 3 choose the respective colored cube.
fn select_target(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<TransformGizmoState>,
    targets: Query<(Entity, &TargetIndex), With<TransformGizmoTarget>>,
) {
    let mut desired_index = None;

    if keys.just_pressed(KeyCode::Digit1) {
        desired_index = Some(1);
    } else if keys.just_pressed(KeyCode::Digit2) {
        desired_index = Some(2);
    } else if keys.just_pressed(KeyCode::Digit3) {
        desired_index = Some(3);
    }

    let Some(index) = desired_index else {
        return;
    };

    for (entity, target_index) in &targets {
        if target_index.0 == index {
            state.active_target = Some(entity);
            break;
        }
    }
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

fn update_hud(state: Res<TransformGizmoState>, mut query: Query<&mut Text, With<GizmoHud>>) {
    let Some(mut text) = query.iter_mut().next() else {
        return;
    };

    let value = format!(
        "Transform Gizmo\nMode:  {mode}\nSpace: {space}\n\nControls:\nT - Translate\nR - Rotate\nS - Scale\nQ - Toggle World/Local\n1/2/3 - Switch active target\nLMB drag axis/ring to manipulate",
        mode = state.mode,
        space = state.space,
    );

    text.0 = value;
}
