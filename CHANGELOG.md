# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025

### Added

- Transform gizmo plugin for Bevy 0.17.x
- Translation handles (axis arrows and plane rectangles)
- Rotation handles (arc segments around each axis)
- Scale handles (axis cubes and uniform scale square)
- World and Local coordinate space support
- Snap-to-grid for translation, rotation, and scale
- Fully customizable styling via `TransformGizmoStyle` resource
- Examples:
  - `single_entity` - Basic single-entity manipulation
  - `multi_gizmos` - Switching between multiple gizmo targets
  - `multiple_entities` - Multi-entity selection with shared pivot
