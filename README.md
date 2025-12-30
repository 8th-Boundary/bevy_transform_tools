# bevy_transform_tools
# ![Bevy Transform Tools](./docs/_media/image.png)
[![Crates.io](https://img.shields.io/crates/v/bevy_transform_tools.svg)](https://crates.io/crates/bevy_transform_tools)
[![Docs.rs](https://docs.rs/bevy_transform_tools/badge.svg)](https://docs.rs/bevy_transform_tools)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A transform gizmo plugin for [Bevy](https://bevyengine.org/) that provides interactive translation, rotation, and scale handles for 3D entities.

## Features

- **Translation** - Move entities along axes (arrows) or planes (rectangles)
- **Rotation** - Rotate entities around any axis (arc handles)
- **Scaling** - Scale entities per-axis (cubes) or uniformly (center square)
- **Coordinate Spaces** - World or local space manipulation
- **Snap-to-Grid** - Optional snapping for translation, rotation, and scale
- **Customizable** - Full control over colors, sizes, and visibility

## Bevy Compatibility

| bevy_transform_tools | Bevy   |
|---------------------|--------|
| 0.1                 | 0.17   |

## Installation

```toml
[dependencies]
bevy_transform_tools = "0.1"
```

## Quick Start

```rust
use bevy::prelude::*;
use bevy_transform_tools::{
    TransformGizmoPlugin, TransformGizmoCamera, TransformGizmoTarget, GizmoActive,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TransformGizmoPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // Camera with gizmo support
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        TransformGizmoCamera,
    ));

    // Entity with active gizmo - just add GizmoActive!
    commands.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0),
        TransformGizmoTarget,
        GizmoActive,
    ));
}
```

## Switching Selection

To change which entity has the gizmo, move the `GizmoActive` component:

```rust
fn switch_target(mut commands: Commands, old: Entity, new: Entity) {
    commands.entity(old).remove::<GizmoActive>();
    commands.entity(new).insert(GizmoActive);
}
```

## Configuration

### TransformGizmoState

Control the gizmo mode:

```rust
fn switch_mode(mut state: ResMut<TransformGizmoState>) {
    state.mode = TransformGizmoMode::Rotate;
    state.space = TransformGizmoSpace::World;
}
```

### TransformGizmoStyle

Customize appearance:

```rust
fn customize_style(mut style: ResMut<TransformGizmoStyle>) {
    style.show_rotate = false;
    style.axis_length = 3.0;
    style.line_width = 2.0;
}
```

### TransformGizmoSnap

Enable snap-to-grid:

```rust
fn enable_snapping(mut snap: ResMut<TransformGizmoSnap>) {
    snap.translate = AxisSnap::uniform(0.5);
    snap.rotate = AxisSnap::uniform(15f32.to_radians());
}
```

## Examples

```bash
cargo run --example single_entity      # Basic usage
cargo run --example multi_gizmos       # Switching targets
cargo run --example multiple_entities  # Multi-selection with pivot
```

## License

MIT License - see [LICENSE](LICENSE) for details.
