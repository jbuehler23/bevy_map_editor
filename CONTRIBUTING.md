# Contributing to bevy_map_editor

Thank you for your interest in contributing! This document provides guidelines and information for contributors.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/bevy_map_editor.git`
3. Create a branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests and checks (see below)
6. Commit and push
7. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.76 or later
- Cargo

### Building

```bash
# Build all crates
cargo build --workspace

# Run the editor
cargo run -p bevy_map_editor

# Run with runtime rendering
cargo run -p bevy_map_editor --features runtime
```

### Running Tests

```bash
cargo test --workspace
```

### Code Quality

Before submitting a PR, ensure your code passes all checks:

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace -- -D warnings

# Run tests
cargo test --workspace
```

## Project Structure

```
bevy_map_editor/
├── crates/
│   ├── bevy_map/           # Main re-export crate
│   ├── bevy_map_core/      # Core data types (Level, Layer, Tileset)
│   ├── bevy_map_editor/    # Visual editor (egui UI)
│   ├── bevy_map_runtime/   # Runtime rendering (bevy_ecs_tilemap)
│   ├── bevy_map_autotile/  # Wang tile autotiling
│   ├── bevy_map_animation/ # Sprite animations
│   ├── bevy_map_dialogue/  # Dialogue trees
│   ├── bevy_map_derive/    # Proc macros
│   └── bevy_map_schema/    # Entity validation
├── examples/               # Example projects
└── docs/                   # Documentation and images
```

## Crate Dependencies

When adding features, be mindful of the dependency order:

```
Level 0 (no internal deps): bevy_map_derive, bevy_map_animation, bevy_map_dialogue
Level 1: bevy_map_core (depends on animation, dialogue)
Level 2: bevy_map_autotile, bevy_map_schema (depend on core)
Level 3: bevy_map_runtime (depends on core, autotile, animation, dialogue)
Level 4: bevy_map_editor, bevy_map (depend on everything)
```

## Pull Request Guidelines

### PR Checklist

- [ ] Code compiles without warnings (`cargo clippy --workspace`)
- [ ] Code is formatted (`cargo fmt --all`)
- [ ] Tests pass (`cargo test --workspace`)
- [ ] New features include tests where applicable
- [ ] Documentation is updated if needed
- [ ] Commit messages are clear and descriptive

### Commit Messages

Use clear, descriptive commit messages:

```
feat: add tile flipping support with X/Y hotkeys
fix: resolve file locking issue on Windows when syncing assets
docs: update README with new keyboard shortcuts
refactor: simplify terrain painting logic
```

### PR Description

Please include:
- **What** the PR does
- **Why** the change is needed
- **How** to test the changes
- Screenshots/GIFs for UI changes

## Reporting Issues

### Bug Reports

Use the bug report template and include:
- Steps to reproduce
- Expected vs actual behavior
- System information (OS, Rust version)
- Screenshots if applicable

### Feature Requests

Use the feature request template and include:
- Clear description of the feature
- Use case / motivation
- Any implementation ideas (optional)

## Code Style

- Follow standard Rust conventions
- Use `rustfmt` for formatting
- Keep functions focused and reasonably sized
- Add comments for complex logic
- Use descriptive variable and function names

## Adding New Features

1. **Discuss first**: For large features, open an issue to discuss the approach
2. **Keep PRs focused**: One feature per PR when possible
3. **Update docs**: Add/update documentation for new features
4. **Add examples**: Consider adding an example if the feature is significant

## Questions?

Feel free to open an issue for questions or join discussions on existing issues.

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).
