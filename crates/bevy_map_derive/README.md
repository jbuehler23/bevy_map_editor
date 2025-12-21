# bevy_map_derive

Proc macros for the bevy_map_editor ecosystem.

Part of [bevy_map_editor](https://github.com/jbuehler23/bevy_map_editor).

## `#[derive(MapEntity)]`

Automatically implements entity spawning from map data.

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;

#[derive(Component, MapEntity)]
#[map_entity(type_name = "Enemy")]
pub struct Enemy {
    #[map_prop]
    pub name: String,

    #[map_prop(default = 100)]
    pub health: i32,

    #[map_prop(name = "patrol_path")]
    pub path: Option<String>,

    #[map_sprite("sprite")]
    pub sprite_handle: Option<Handle<Image>>,
}
```

## Attributes

### Container: `#[map_entity(...)]`

| Attribute            | Required | Description                        |
|----------------------|----------|------------------------------------|
| `type_name = "Name"` | Yes      | Entity type name as used in editor |

### Field: `#[map_prop(...)]`

| Attribute            | Description                                  |
|----------------------|----------------------------------------------|
| `name = "prop_name"` | Override property name (default: field name) |
| `default = value`    | Default value if property missing            |

### Field: `#[map_sprite(...)]`

Inject sprite handle from map data.

| Usage                   | Description                     |
|-------------------------|---------------------------------|
| `#[map_sprite]`         | Use field name as property name |
| `#[map_sprite("name")]` | Use specified property name     |

Field must be `Option<Handle<Image>>`.

## Complete Example

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;

#[derive(Component, MapEntity)]
#[map_entity(type_name = "Player")]
pub struct Player {
    #[map_prop]
    pub name: String,

    #[map_prop(default = 5.0)]
    pub speed: f32,

    #[map_sprite("player_sprite")]
    pub sprite: Option<Handle<Image>>,
}

// Register in your app builder
App::new()
    .add_plugins(MapRuntimePlugin)
    .register_map_entity::<Player>()
    // ... other setup
```

## License

MIT OR Apache-2.0
