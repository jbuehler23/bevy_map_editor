//! Runtime map rendering via bevy_ecs_tilemap
//!
//! This crate provides Bevy integration for loading and rendering maps
//! created with bevy_map_editor using bevy_ecs_tilemap.
//!
//! # Features
//! - Asset loader for `.map.json` files with hot-reload support
//! - bevy_ecs_tilemap-based GPU rendering
//! - Runtime terrain modification support via autotile integration
//! - Automatic entity spawning with derive macros
//!
//! # Quick Start (Asset-Based Loading with Hot-Reload)
//!
//! This is the recommended approach for most games:
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use bevy_map_runtime::{MapRuntimePlugin, MapHandle};
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(MapRuntimePlugin)
//!         .add_systems(Startup, load_map)
//!         .run();
//! }
//!
//! fn load_map(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     // Load map as a Bevy asset - supports hot-reload!
//!     commands.spawn(MapHandle(asset_server.load("maps/level1.map.json")));
//! }
//! ```
//!
//! To enable hot-reloading during development:
//! ```bash
//! cargo run --features bevy/file_watcher
//! ```
//!
//! # Manual Spawning (Advanced)
//!
//! For more control over spawning:
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use bevy_map_runtime::{MapRuntimePlugin, SpawnMapProjectEvent, TilesetTextures};
//! use bevy_map_core::MapProject;
//!
//! fn load_map(
//!     mut commands: Commands,
//!     asset_server: Res<AssetServer>,
//!     mut spawn_events: MessageWriter<SpawnMapProjectEvent>,
//! ) {
//!     let json = include_str!("../assets/maps/level1.map.json");
//!     let project: MapProject = serde_json::from_str(json).unwrap();
//!
//!     let mut textures = TilesetTextures::new();
//!     textures.load_from_project(&project, &asset_server);
//!
//!     spawn_events.send(SpawnMapProjectEvent {
//!         project,
//!         textures,
//!         transform: Transform::default(),
//!     });
//! }
//! ```

use bevy::asset::AssetEvent;
use bevy::ecs::message::{Message, MessageReader, MessageWriter};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_map_core::MapProject;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

/// Convert an absolute file path to a relative asset path.
///
/// This handles paths saved by the editor which may be absolute (e.g.,
/// "C:/Users/.../assets/tiles/tileset.png") and converts them to relative
/// paths that Bevy's AssetServer can load (e.g., "tiles/tileset.png").
///
/// The function looks for an "assets" directory in the path and returns
/// everything after it. If no "assets" directory is found, returns the
/// original path.
fn normalize_asset_path(path: &str) -> String {
    // Normalize path separators to forward slashes
    let normalized = path.replace('\\', "/");

    // Look for "/assets/" marker in the path (case-insensitive)
    let lower = normalized.to_lowercase();
    if let Some(idx) = lower.find("/assets/") {
        // Return everything after "/assets/"
        return normalized[idx + 8..].to_string();
    }

    // If path starts with "assets/", strip it
    if lower.starts_with("assets/") {
        return normalized[7..].to_string();
    }

    // Check if it's already a relative path (no drive letter or leading slash)
    let path_obj = Path::new(&normalized);
    if !path_obj.has_root() && !normalized.contains(':') {
        return normalized;
    }

    // If we couldn't find an assets directory, try to extract just the filename
    // as a fallback (this helps in edge cases)
    if let Some(file_name) = Path::new(&normalized).file_name() {
        if let Some(name) = file_name.to_str() {
            warn!(
                "Could not find 'assets' directory in path '{}', using filename '{}'",
                path, name
            );
            return name.to_string();
        }
    }

    // Last resort: return original path and let Bevy handle the error
    normalized
}

// Re-export core types
pub use bevy_map_animation;
pub use bevy_map_autotile;
pub use bevy_map_core;
pub use bevy_map_dialogue;

pub mod camera;
pub mod collision;
pub mod entity_input;
pub mod entity_physics;
pub mod entity_registry;
pub mod entity_sprite;
pub mod loader;
pub mod render;

// Re-export commonly used types
pub use camera::{clamp_camera_to_bounds, setup_camera_bounds_from_map, CameraBounds};
pub use collision::{MapCollider, MapCollisionPlugin};
pub use entity_input::{
    CustomInput, EntityInputSpawned, MapEntityInputPlugin, PlatformerInput, TopDownInput,
    TwinStickInput,
};
pub use entity_physics::{EntityPhysicsSpawned, MapEntityPhysicsPlugin};
pub use entity_registry::{
    attach_dialogues, Dialogue, EntityProperties, EntityRegistry, MapEntityExt, MapEntityMarker,
    MapEntityType,
};
pub use entity_sprite::{EntitySpriteSetup, EntitySpriteSpawned, MapEntitySpritePlugin};
pub use loader::{MapLoadError, MapProjectLoader};
pub use render::{complete_sprite_loads, spawn_sprite_components, SpriteSlot};

// Re-export key dialogue types for convenience
pub use bevy_map_dialogue::{
    DialogueChoice, DialogueChoiceEvent, DialogueEndEvent, DialogueHandle, DialogueNode,
    DialogueNodeType, DialogueRunner, DialogueTree, StartDialogueEvent,
};

// Re-export key animation types for convenience
pub use bevy_map_animation::{
    AnimatedSprite, AnimationCustomEvent, AnimationDef, AnimationEventExt, AnimationParticleEvent,
    AnimationSoundEvent, AnimationTrigger, AnimationTriggerEvent, AnimationTriggerRegistry,
    AnimationTriggerType, AnimationTriggered, AnimationWindow, AnimationWindowChanged,
    AnimationWindowEvent, AnimationWindowRegistry, AnimationWindowType, LoopMode,
    SpriteAnimationPlugin, SpriteData, TriggerPayload, WindowPhase, WindowTracker,
};

/// Plugin for runtime map rendering
///
/// This plugin provides:
/// - Asset loading for `.map.json` files
/// - Automatic map spawning when `MapHandle` components are added
/// - Hot-reload support when using Bevy's `file_watcher` feature
/// - Manual spawning via `SpawnMapEvent` and `SpawnMapProjectEvent`
pub struct MapRuntimePlugin;

impl Plugin for MapRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapPlugin)
            .add_plugins(bevy_map_dialogue::DialoguePlugin)
            .add_plugins(bevy_map_animation::SpriteAnimationPlugin)
            // Asset loading
            .init_asset::<MapProject>()
            .init_asset_loader::<MapProjectLoader>()
            // Resources
            .init_resource::<EntityRegistry>()
            .init_resource::<MapDialogues>()
            // Events
            .add_message::<SpawnMapEvent>()
            .add_message::<SpawnMapProjectEvent>()
            .add_message::<MapSpawnedEvent>()
            // Systems
            .add_systems(Update, handle_spawn_map_events)
            .add_systems(Update, handle_spawn_map_project_events)
            .add_systems(
                Update,
                (
                    initialize_map_handles,
                    handle_map_handle_spawning,
                    handle_map_hot_reload,
                )
                    .chain(),
            )
            // Sprite spawning systems
            .add_systems(Update, spawn_sprite_components)
            .add_systems(Update, complete_sprite_loads)
            // Dialogue attachment system
            .add_systems(Update, attach_dialogues)
            // Camera bounds systems
            .add_systems(Update, setup_camera_bounds_from_map)
            .add_systems(PostUpdate, clamp_camera_to_bounds)
            // Animated sprite auto-loading systems (opt-in)
            .add_systems(
                Update,
                (
                    initialize_animated_sprite_handles,
                    handle_animated_sprite_loading,
                )
                    .chain(),
            )
            // Dialogue tree auto-loading systems (opt-in)
            .add_systems(
                Update,
                (
                    initialize_dialogue_tree_handles,
                    handle_dialogue_tree_loading,
                )
                    .chain(),
            );
    }
}

// ============================================================================
// Asset-based map loading (recommended approach)
// ============================================================================

/// Component for loading maps via the Bevy asset system
///
/// Attach this component to an entity to load and spawn a map. The map will
/// be automatically spawned once the asset is loaded, and will be respawned
/// if the asset changes (hot-reload).
///
/// # Example
///
/// ```rust,ignore
/// fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
///     commands.spawn((
///         MapHandle(asset_server.load("maps/level1.map.json")),
///         Transform::from_xyz(100.0, 0.0, 0.0),  // Optional: offset the map
///     ));
/// }
/// ```
#[derive(Component)]
pub struct MapHandle(pub Handle<MapProject>);

/// Marker component for the root entity of a spawned map
///
/// This is added automatically when a map is spawned. It tracks the source
/// asset handle for hot-reload support.
#[derive(Component)]
pub struct MapRoot {
    /// Handle to the source MapProject asset
    pub handle: Handle<MapProject>,
    /// Cached tileset textures for this map
    pub textures: TilesetTextures,
}

/// Internal state tracking for MapHandle entities
#[derive(Component, Default)]
struct MapHandleState {
    /// Whether we've started loading textures
    loading_textures: bool,
    /// Cached tileset textures
    textures: Option<TilesetTextures>,
    /// Whether the map has been spawned
    spawned: bool,
}

/// System that initializes newly added MapHandle components
fn initialize_map_handles(mut commands: Commands, query: Query<Entity, Added<MapHandle>>) {
    for entity in query.iter() {
        commands.entity(entity).insert(MapHandleState::default());
    }
}

/// System that spawns maps when MapHandle components are added
fn handle_map_handle_spawning(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    map_assets: Res<Assets<MapProject>>,
    mut query: Query<(Entity, &MapHandle, &mut MapHandleState, Option<&Transform>)>,
    entity_registry: Res<EntityRegistry>,
    mut map_dialogues: ResMut<MapDialogues>,
) {
    for (entity, map_handle, mut state, _transform) in query.iter_mut() {
        // Check if asset is loaded
        let Some(project) = map_assets.get(&map_handle.0) else {
            continue;
        };

        // Start loading textures if we haven't
        if !state.loading_textures {
            info!(
                "MapProject '{}' loaded, queueing texture loads...",
                project.level.name
            );
            let mut textures = TilesetTextures::new();
            textures.load_from_project(project, &asset_server);
            info!(
                "Queued {} tileset images, {} sprite sheets for loading",
                textures.images.len(),
                textures.sprite_sheet_images.len()
            );
            state.textures = Some(textures);
            state.loading_textures = true;
        }

        // Check if all textures are loaded
        let Some(textures) = &state.textures else {
            continue;
        };

        if !textures.all_loaded(&asset_server) {
            // Log loading state periodically (only once per second to avoid spam)
            static LAST_LOG: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let last = LAST_LOG.load(std::sync::atomic::Ordering::Relaxed);
            if now > last {
                LAST_LOG.store(now, std::sync::atomic::Ordering::Relaxed);
                textures.log_loading_state(&asset_server);
            }
            continue;
        }

        // Don't spawn if already spawned
        if state.spawned {
            continue;
        }

        info!(
            "Spawning map '{}' with {} layers, {} tilesets",
            project.level.name,
            project.level.layers.len(),
            project.tilesets.len()
        );

        // Load dialogues from the project
        map_dialogues.load_from_project(project);

        let map_entity = spawn_map_project(
            &mut commands,
            project,
            textures,
            Transform::default(), // Map is relative to parent
            Some(&entity_registry),
        );

        // Add MapRoot marker and make it a child
        commands.entity(map_entity).insert(MapRoot {
            handle: map_handle.0.clone(),
            textures: textures.clone(),
        });

        commands.entity(entity).add_child(map_entity);

        state.spawned = true;
        info!("Spawned map: {}", project.level.name);
    }
}

/// System that handles hot-reloading of maps when assets change
fn handle_map_hot_reload(
    mut commands: Commands,
    mut asset_events: MessageReader<AssetEvent<MapProject>>,
    mut query: Query<(Entity, &MapHandle, &mut MapHandleState)>,
    children_query: Query<&Children>,
    map_root_query: Query<(Entity, &MapRoot)>,
) {
    for event in asset_events.read() {
        let AssetEvent::Modified { id } = event else {
            continue;
        };

        // Find MapHandle entities using this asset
        for (entity, map_handle, mut state) in query.iter_mut() {
            if map_handle.0.id() != *id {
                continue;
            }

            info!("Hot-reloading map asset");

            // Find and despawn existing map root
            if let Ok(children) = children_query.get(entity) {
                for child in children.iter() {
                    if map_root_query.get(child).is_ok() {
                        commands.entity(child).despawn();
                    }
                }
            }

            // Reset state to trigger respawn
            state.spawned = false;
            state.loading_textures = false;
            state.textures = None;
        }
    }
}

/// Extension trait for spawning maps via commands
pub trait MapCommandsExt {
    /// Spawn a map from an asset path
    ///
    /// Returns the entity that will contain the map once loaded.
    fn spawn_map(&mut self, asset_server: &AssetServer, path: impl Into<String>) -> Entity;
}

impl MapCommandsExt for Commands<'_, '_> {
    fn spawn_map(&mut self, asset_server: &AssetServer, path: impl Into<String>) -> Entity {
        self.spawn((
            MapHandle(asset_server.load(path.into())),
            MapHandleState::default(),
            Transform::default(),
            Visibility::default(),
        ))
        .id()
    }
}

/// Manages loaded tileset and sprite sheet textures for a map
///
/// This provides a convenient way to map tileset IDs to their loaded texture handles,
/// properly handling multi-image tilesets and sprite sheets.
#[derive(Debug, Clone, Default)]
pub struct TilesetTextures {
    /// Map from (tileset_id, image_index) to texture handle
    images: HashMap<(Uuid, usize), Handle<Image>>,
    /// Map from sprite_sheet_id to texture handle
    sprite_sheet_images: HashMap<Uuid, Handle<Image>>,
    /// Tile size from the project (cached for convenience)
    pub tile_size: f32,
}

impl TilesetTextures {
    /// Create a new empty TilesetTextures
    pub fn new() -> Self {
        Self::default()
    }

    /// Load all textures from a MapProject using the asset server
    pub fn load_from_project(
        &mut self,
        project: &bevy_map_core::MapProject,
        asset_server: &AssetServer,
    ) {
        // Load tileset images
        for (tileset_id, image_index, path) in project.image_paths() {
            // Convert absolute paths to relative asset paths
            let asset_path = normalize_asset_path(path);
            let handle = asset_server.load(asset_path);
            self.images.insert((tileset_id, image_index), handle);
        }

        // Load sprite sheet images
        for (sprite_sheet_id, path) in project.sprite_sheet_paths() {
            // Convert absolute paths to relative asset paths
            let asset_path = normalize_asset_path(path);
            let handle = asset_server.load(asset_path);
            self.sprite_sheet_images.insert(sprite_sheet_id, handle);
        }

        // Get tile size from the first tileset
        if let Some(tileset) = project.tilesets.values().next() {
            self.tile_size = tileset.tile_size as f32;
        }
    }

    /// Get texture handle for a specific tileset and image index
    pub fn get(&self, tileset_id: Uuid, image_index: usize) -> Option<&Handle<Image>> {
        self.images.get(&(tileset_id, image_index))
    }

    /// Insert a texture handle manually
    pub fn insert(&mut self, tileset_id: Uuid, image_index: usize, handle: Handle<Image>) {
        self.images.insert((tileset_id, image_index), handle);
    }

    /// Get texture handle for a sprite sheet
    pub fn get_sprite_sheet(&self, sprite_sheet_id: Uuid) -> Option<&Handle<Image>> {
        self.sprite_sheet_images.get(&sprite_sheet_id)
    }

    /// Insert a sprite sheet texture handle manually
    pub fn insert_sprite_sheet(&mut self, sprite_sheet_id: Uuid, handle: Handle<Image>) {
        self.sprite_sheet_images.insert(sprite_sheet_id, handle);
    }

    /// Check if all textures (tilesets and sprite sheets) are loaded
    pub fn all_loaded(&self, asset_server: &AssetServer) -> bool {
        use bevy::asset::LoadState;

        let tilesets_loaded = self.images.values().all(|handle| {
            match asset_server.get_load_state(handle.id()) {
                Some(LoadState::Loaded) => true,
                Some(LoadState::Failed(_)) => {
                    // Treat failed as "loaded" so we don't block forever
                    // The error will be logged elsewhere
                    true
                }
                _ => false,
            }
        });

        let sprite_sheets_loaded = self.sprite_sheet_images.values().all(|handle| {
            matches!(
                asset_server.get_load_state(handle.id()),
                Some(LoadState::Loaded) | Some(LoadState::Failed(_))
            )
        });

        tilesets_loaded && sprite_sheets_loaded
    }

    /// Log the current loading state of all textures (for debugging)
    pub fn log_loading_state(&self, asset_server: &AssetServer) {
        use bevy::asset::LoadState;

        for ((tileset_id, image_index), handle) in &self.images {
            let state = asset_server.get_load_state(handle.id());
            match state {
                Some(LoadState::Loaded) => {}
                Some(LoadState::Failed(ref err)) => {
                    warn!(
                        "Tileset {:?} image {}: FAILED - {}",
                        tileset_id, image_index, err
                    );
                }
                Some(LoadState::Loading) => {
                    info!(
                        "Tileset {:?} image {}: still loading...",
                        tileset_id, image_index
                    );
                }
                state => {
                    info!(
                        "Tileset {:?} image {}: state {:?}",
                        tileset_id, image_index, state
                    );
                }
            }
        }

        for (sprite_sheet_id, handle) in &self.sprite_sheet_images {
            let state = asset_server.get_load_state(handle.id());
            match state {
                Some(LoadState::Loaded) => {}
                Some(LoadState::Failed(ref err)) => {
                    warn!("Sprite sheet {:?}: FAILED - {}", sprite_sheet_id, err);
                }
                _ => {
                    info!("Sprite sheet {:?}: state {:?}", sprite_sheet_id, state);
                }
            }
        }
    }
}

/// Resource storing all dialogue trees from the loaded map
///
/// This is automatically populated when a map is loaded via `MapHandle` or
/// `SpawnMapProjectEvent`. Dialogue trees can be referenced by their ID.
///
/// # Example
///
/// ```rust,ignore
/// fn check_dialogue(
///     map_dialogues: Res<MapDialogues>,
/// ) {
///     if let Some(tree) = map_dialogues.get("merchant_greeting") {
///         // Use the dialogue tree
///     }
/// }
/// ```
#[derive(Resource, Default, Debug, Clone)]
pub struct MapDialogues {
    /// Dialogue trees keyed by their ID
    pub dialogues: HashMap<String, bevy_map_dialogue::DialogueTree>,
}

impl MapDialogues {
    /// Get a dialogue tree by ID
    pub fn get(&self, id: &str) -> Option<&bevy_map_dialogue::DialogueTree> {
        self.dialogues.get(id)
    }

    /// Check if a dialogue exists
    pub fn contains(&self, id: &str) -> bool {
        self.dialogues.contains_key(id)
    }

    /// Get all dialogue IDs
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.dialogues.keys().map(|s| s.as_str())
    }

    /// Load dialogues from a MapProject
    pub fn load_from_project(&mut self, project: &MapProject) {
        self.dialogues = project.dialogues.clone();
        info!(
            "Loaded {} dialogue(s) from map project",
            self.dialogues.len()
        );
    }

    /// Clear all loaded dialogues
    pub fn clear(&mut self) {
        self.dialogues.clear();
    }
}

// ============================================================================
// Animated Sprite Auto-Loading (optional convenience component)
// ============================================================================

/// Optional component for auto-loading animated sprites from a map project.
///
/// When added to an entity, this component will:
/// 1. Wait for the MapProject to load
/// 2. Find the sprite sheet by name
/// 3. Load the sprite sheet texture
/// 4. Add AnimatedSprite and Sprite components automatically
///
/// This is an **opt-in** convenience feature. For full control, you can
/// still load sprites manually as shown in the manual loading examples.
///
/// # Example
///
/// ```rust,ignore
/// fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
///     // ONE LINE: AnimatedSpriteHandle handles all the loading
///     commands.spawn((
///         AnimatedSpriteHandle::new(
///             asset_server.load("maps/demo.map.json"),
///             "player_sprite",  // sprite sheet name in editor
///             "idle",           // initial animation to play
///         ),
///         Transform::from_xyz(100.0, 0.0, 0.0),
///     ));
/// }
/// ```
#[derive(Component)]
pub struct AnimatedSpriteHandle {
    /// Handle to the MapProject asset containing the sprite sheet
    pub map: Handle<MapProject>,
    /// Name of the sprite sheet in the project
    pub sprite_sheet_name: String,
    /// Initial animation to play
    pub initial_animation: String,
    /// Optional scale factor for the sprite (multiplies frame size)
    pub scale: Option<f32>,
}

impl AnimatedSpriteHandle {
    /// Create a new AnimatedSpriteHandle
    ///
    /// # Arguments
    /// * `map` - Handle to the MapProject asset
    /// * `sprite_sheet` - Name of the sprite sheet in the project
    /// * `animation` - Name of the initial animation to play
    pub fn new(map: Handle<MapProject>, sprite_sheet: &str, animation: &str) -> Self {
        Self {
            map,
            sprite_sheet_name: sprite_sheet.to_string(),
            initial_animation: animation.to_string(),
            scale: None,
        }
    }

    /// Set a scale factor for the sprite
    ///
    /// The scale multiplies the frame size, e.g., scale=4.0 with 32x32 frames
    /// gives a 128x128 sprite on screen.
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = Some(scale);
        self
    }
}

/// Internal state for AnimatedSpriteHandle loading
#[derive(Component, Default)]
struct AnimatedSpriteHandleState {
    /// Whether we've started loading the texture
    texture_loading: bool,
    /// Handle to the texture being loaded
    texture_handle: Option<Handle<Image>>,
    /// Handle to the SpriteData asset
    sprite_data_handle: Option<Handle<bevy_map_animation::SpriteData>>,
    /// Cached frame size (width, height)
    frame_size: Option<(u32, u32)>,
    /// Whether loading is complete
    completed: bool,
}

/// System that initializes newly added AnimatedSpriteHandle components
fn initialize_animated_sprite_handles(
    mut commands: Commands,
    query: Query<Entity, Added<AnimatedSpriteHandle>>,
) {
    for entity in query.iter() {
        commands
            .entity(entity)
            .insert(AnimatedSpriteHandleState::default());
    }
}

/// System that handles automatic loading of animated sprites
fn handle_animated_sprite_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    map_assets: Res<Assets<MapProject>>,
    mut sprite_data_assets: ResMut<Assets<bevy_map_animation::SpriteData>>,
    mut query: Query<(
        Entity,
        &AnimatedSpriteHandle,
        &mut AnimatedSpriteHandleState,
    )>,
) {
    use bevy::asset::LoadState;

    for (entity, handle, mut state) in query.iter_mut() {
        if state.completed {
            continue;
        }

        // Wait for MapProject to load
        let Some(project) = map_assets.get(&handle.map) else {
            continue;
        };

        // Find sprite sheet by name
        let Some(sprite_data) = project.sprite_sheet_by_name(&handle.sprite_sheet_name) else {
            warn!(
                "Sprite sheet '{}' not found in project",
                handle.sprite_sheet_name
            );
            state.completed = true;
            continue;
        };

        // Start loading texture if needed
        if !state.texture_loading {
            // Convert absolute paths to relative asset paths
            let asset_path = normalize_asset_path(&sprite_data.sheet_path);
            let texture: Handle<Image> = asset_server.load(asset_path);
            state.texture_handle = Some(texture);
            state.sprite_data_handle = Some(sprite_data_assets.add(sprite_data.clone()));
            state.frame_size = Some((sprite_data.frame_width, sprite_data.frame_height));
            state.texture_loading = true;
        }

        // Check if texture is loaded
        let Some(texture_handle) = &state.texture_handle else {
            continue;
        };
        if !matches!(
            asset_server.get_load_state(texture_handle.id()),
            Some(LoadState::Loaded)
        ) {
            continue;
        }

        // All loaded - spawn the animated sprite components
        let sprite_data_handle = state.sprite_data_handle.clone().unwrap();
        let (frame_w, frame_h) = state.frame_size.unwrap();

        let mut animated = bevy_map_animation::AnimatedSprite::new(sprite_data_handle);
        animated.play(&handle.initial_animation);

        let custom_size = handle
            .scale
            .map(|s| Vec2::new(frame_w as f32 * s, frame_h as f32 * s));

        commands.entity(entity).insert((
            Sprite {
                image: texture_handle.clone(),
                rect: Some(Rect::new(0.0, 0.0, frame_w as f32, frame_h as f32)),
                custom_size,
                ..default()
            },
            animated,
        ));

        state.completed = true;
        info!(
            "Auto-loaded animated sprite: {} (animation: {})",
            handle.sprite_sheet_name, handle.initial_animation
        );
    }
}

// ============================================================================
// Dialogue Tree Auto-Loading (optional convenience component)
// ============================================================================

/// Optional component for auto-loading dialogue trees from a map project.
///
/// When added to an entity, this component will:
/// 1. Wait for the MapProject to load
/// 2. Find the dialogue tree by name
/// 3. Add a DialogueHandle component with the loaded tree
///
/// This is an **opt-in** convenience feature. For full control, you can
/// still load dialogues manually.
///
/// # Example
///
/// ```rust,ignore
/// fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
///     // ONE LINE: DialogueTreeHandle handles all the loading
///     commands.spawn((
///         DialogueTreeHandle::new(
///             asset_server.load("maps/demo.map.json"),
///             "merchant_greeting",  // dialogue name in editor
///         ),
///     ));
/// }
/// ```
#[derive(Component)]
pub struct DialogueTreeHandle {
    /// Handle to the MapProject asset containing the dialogue
    pub map: Handle<MapProject>,
    /// Name of the dialogue tree in the project
    pub dialogue_name: String,
}

impl DialogueTreeHandle {
    /// Create a new DialogueTreeHandle
    ///
    /// # Arguments
    /// * `map` - Handle to the MapProject asset
    /// * `dialogue_name` - Name of the dialogue tree in the project
    pub fn new(map: Handle<MapProject>, dialogue_name: &str) -> Self {
        Self {
            map,
            dialogue_name: dialogue_name.to_string(),
        }
    }
}

/// Internal state for DialogueTreeHandle loading
#[derive(Component, Default)]
struct DialogueTreeHandleState {
    /// Whether loading is complete
    completed: bool,
}

/// System that initializes newly added DialogueTreeHandle components
fn initialize_dialogue_tree_handles(
    mut commands: Commands,
    query: Query<Entity, Added<DialogueTreeHandle>>,
) {
    for entity in query.iter() {
        commands
            .entity(entity)
            .insert(DialogueTreeHandleState::default());
    }
}

/// System that handles automatic loading of dialogue trees
fn handle_dialogue_tree_loading(
    mut commands: Commands,
    map_assets: Res<Assets<MapProject>>,
    mut dialogue_assets: ResMut<Assets<bevy_map_dialogue::DialogueTree>>,
    mut query: Query<(Entity, &DialogueTreeHandle, &mut DialogueTreeHandleState)>,
) {
    for (entity, handle, mut state) in query.iter_mut() {
        if state.completed {
            continue;
        }

        // Wait for MapProject to load
        let Some(project) = map_assets.get(&handle.map) else {
            continue;
        };

        // Find dialogue by name
        let Some(dialogue) = project.dialogue_by_name(&handle.dialogue_name) else {
            warn!("Dialogue '{}' not found in project", handle.dialogue_name);
            state.completed = true;
            continue;
        };

        // Add dialogue to assets and attach DialogueHandle component
        let dialogue_handle = dialogue_assets.add(dialogue.clone());
        commands
            .entity(entity)
            .insert(DialogueHandle(dialogue_handle));

        state.completed = true;
        info!("Auto-loaded dialogue: {}", handle.dialogue_name);
    }
}

/// Event to spawn a map from a Level
#[derive(Message)]
pub struct SpawnMapEvent {
    /// The level to spawn
    pub level: bevy_map_core::Level,
    /// Transform for the map entity
    pub transform: Transform,
    /// Tile size in pixels (used for rendering)
    pub tile_size: f32,
    /// Tileset textures (indexed by tileset order in level)
    pub tileset_textures: Vec<Handle<Image>>,
}

/// Event emitted when a map has been spawned
#[derive(Message)]
pub struct MapSpawnedEvent {
    /// Entity of the spawned map
    pub map_entity: Entity,
}

/// Component marking a runtime map entity
#[derive(Component, Default)]
pub struct RuntimeMap {
    /// Reference to the original level data
    pub level_name: String,
}

/// Component linking a tilemap layer to its source layer index
#[derive(Component)]
pub struct MapLayerIndex(pub usize);

fn handle_spawn_map_events(
    mut commands: Commands,
    mut spawn_events: MessageReader<SpawnMapEvent>,
    mut spawned_events: MessageWriter<MapSpawnedEvent>,
    entity_registry: Res<EntityRegistry>,
) {
    for event in spawn_events.read() {
        let map_entity = spawn_map(
            &mut commands,
            &event.level,
            event.tile_size,
            &event.tileset_textures,
            event.transform,
            Some(&entity_registry),
        );
        spawned_events.write(MapSpawnedEvent { map_entity });
    }
}

/// Spawn a map from a Level with the given tileset textures
///
/// If an `EntityRegistry` is provided, entities from the level will be
/// automatically spawned with the appropriate components.
pub fn spawn_map(
    commands: &mut Commands,
    level: &bevy_map_core::Level,
    tile_size: f32,
    tileset_textures: &[Handle<Image>],
    transform: Transform,
    entity_registry: Option<&EntityRegistry>,
) -> Entity {
    let map_entity = commands
        .spawn((
            RuntimeMap {
                level_name: level.name.clone(),
            },
            transform,
            Visibility::default(),
        ))
        .id();

    // Spawn each tile layer
    for (layer_index, layer) in level.layers.iter().enumerate() {
        if let bevy_map_core::LayerData::Tiles { tiles, .. } = &layer.data {
            if tiles.is_empty() {
                continue;
            }

            // Get the tileset texture for this layer
            let texture_handle = if layer_index < tileset_textures.len() {
                tileset_textures[layer_index].clone()
            } else if !tileset_textures.is_empty() {
                tileset_textures[0].clone()
            } else {
                continue;
            };

            let map_size = TilemapSize {
                x: level.width,
                y: level.height,
            };

            let tilemap_tile_size = TilemapTileSize {
                x: tile_size,
                y: tile_size,
            };

            let grid_size: TilemapGridSize = tilemap_tile_size.into();

            // Create tilemap storage
            let mut tile_storage = TileStorage::empty(map_size);

            let tilemap_entity = commands.spawn_empty().id();

            // Spawn tiles
            for y in 0..level.height {
                for x in 0..level.width {
                    let idx = (y * level.width + x) as usize;
                    if let Some(&Some(tile_index)) = tiles.get(idx) {
                        let tile_pos = TilePos { x, y };
                        let tile_entity = commands
                            .spawn(TileBundle {
                                position: tile_pos,
                                tilemap_id: TilemapId(tilemap_entity),
                                texture_index: TileTextureIndex(tile_index),
                                ..default()
                            })
                            .id();
                        tile_storage.set(&tile_pos, tile_entity);
                    }
                }
            }

            let map_type = TilemapType::Square;

            // Calculate layer z-offset based on layer index
            let layer_z = layer_index as f32 * 0.1;

            commands.entity(tilemap_entity).insert((
                TilemapBundle {
                    grid_size,
                    map_type,
                    size: map_size,
                    storage: tile_storage,
                    texture: TilemapTexture::Single(texture_handle),
                    tile_size: tilemap_tile_size,
                    transform: Transform::from_xyz(0.0, 0.0, layer_z),
                    ..default()
                },
                MapLayerIndex(layer_index),
            ));

            commands.entity(map_entity).add_child(tilemap_entity);
        }
    }

    // Spawn entities if registry is provided
    if let Some(registry) = entity_registry {
        registry.spawn_all(commands, &level.entities, transform);
    }

    map_entity
}

/// Update a tile at runtime
pub fn set_tile(
    commands: &mut Commands,
    tile_storage: &mut TileStorage,
    tilemap_entity: Entity,
    x: u32,
    y: u32,
    tile_index: Option<u32>,
) {
    let tile_pos = TilePos { x, y };

    // Remove existing tile if present
    if let Some(tile_entity) = tile_storage.get(&tile_pos) {
        commands.entity(tile_entity).despawn();
        tile_storage.remove(&tile_pos);
    }

    // Spawn new tile if index provided
    if let Some(index) = tile_index {
        let tile_entity = commands
            .spawn(TileBundle {
                position: tile_pos,
                tilemap_id: TilemapId(tilemap_entity),
                texture_index: TileTextureIndex(index),
                ..default()
            })
            .id();
        tile_storage.set(&tile_pos, tile_entity);
    }
}

// ============================================================================
// MapProject-based spawning (improved ergonomics)
// ============================================================================

/// Event to spawn a map from a MapProject with embedded tileset metadata
///
/// This is the recommended way to spawn maps as it automatically handles
/// tileset texture mapping and multi-image tilesets.
#[derive(Message)]
pub struct SpawnMapProjectEvent {
    /// The map project containing level and tileset data
    pub project: bevy_map_core::MapProject,
    /// Pre-loaded tileset textures
    pub textures: TilesetTextures,
    /// Transform for the map entity
    pub transform: Transform,
}

fn handle_spawn_map_project_events(
    mut commands: Commands,
    mut spawn_events: MessageReader<SpawnMapProjectEvent>,
    mut spawned_events: MessageWriter<MapSpawnedEvent>,
    entity_registry: Res<EntityRegistry>,
    mut map_dialogues: ResMut<MapDialogues>,
) {
    for event in spawn_events.read() {
        // Load dialogues from the project
        map_dialogues.load_from_project(&event.project);

        let map_entity = spawn_map_project(
            &mut commands,
            &event.project,
            &event.textures,
            event.transform,
            Some(&entity_registry),
        );
        spawned_events.write(MapSpawnedEvent { map_entity });
    }
}

/// Spawn a map from a MapProject with proper tileset handling
///
/// This function properly handles:
/// - Multi-image tilesets (tiles referencing correct image by virtual index)
/// - Tileset metadata embedded in the project
/// - Entity spawning via EntityRegistry
///
/// # Example
///
/// ```rust,ignore
/// use bevy_map_runtime::{spawn_map_project, TilesetTextures};
/// use bevy_map_core::MapProject;
///
/// fn spawn_level(
///     mut commands: Commands,
///     asset_server: Res<AssetServer>,
/// ) {
///     // Load project from JSON
///     let json = include_str!("../assets/maps/level1.map.json");
///     let project: MapProject = serde_json::from_str(json).unwrap();
///
///     // Load textures
///     let mut textures = TilesetTextures::new();
///     textures.load_from_project(&project, &asset_server);
///
///     // Spawn map
///     spawn_map_project(&mut commands, &project, &textures, Transform::default(), None);
/// }
/// ```
pub fn spawn_map_project(
    commands: &mut Commands,
    project: &bevy_map_core::MapProject,
    textures: &TilesetTextures,
    transform: Transform,
    entity_registry: Option<&EntityRegistry>,
) -> Entity {
    let level = &project.level;
    let tile_size = textures.tile_size;

    let map_entity = commands
        .spawn((
            RuntimeMap {
                level_name: level.name.clone(),
            },
            transform,
            Visibility::default(),
        ))
        .id();

    // Spawn each tile layer
    for (layer_index, layer) in level.layers.iter().enumerate() {
        info!("Processing layer {}: '{}'", layer_index, layer.name);

        if let bevy_map_core::LayerData::Tiles {
            tileset_id, tiles, ..
        } = &layer.data
        {
            info!(
                "  Layer {} is a tile layer with {} tiles, tileset {}",
                layer_index,
                tiles.len(),
                tileset_id
            );

            if tiles.is_empty() {
                info!("  Layer {} has empty tiles array, skipping", layer_index);
                continue;
            }

            // Get tileset from project
            let Some(tileset) = project.get_tileset(*tileset_id) else {
                warn!(
                    "Layer {} references missing tileset {}",
                    layer_index, tileset_id
                );
                continue;
            };

            info!(
                "  Found tileset '{}' with {} images",
                tileset.name,
                tileset.images.len()
            );

            // For multi-image tilesets, we need to create separate tilemaps per image
            // because bevy_ecs_tilemap uses a single texture per tilemap.
            // Group tiles by which image they belong to.
            let mut tiles_by_image: HashMap<usize, Vec<(u32, u32, u32)>> = HashMap::new();

            for y in 0..level.height {
                for x in 0..level.width {
                    let idx = (y * level.width + x) as usize;
                    if let Some(&Some(virtual_tile_index)) = tiles.get(idx) {
                        if let Some((image_index, local_tile_index)) =
                            tileset.virtual_to_local(virtual_tile_index)
                        {
                            tiles_by_image.entry(image_index).or_default().push((
                                x,
                                y,
                                local_tile_index,
                            ));
                        }
                    }
                }
            }

            // Spawn a tilemap for each image used in this layer
            for (image_index, image_tiles) in tiles_by_image {
                info!(
                    "Layer {}: Spawning {} tiles from tileset {} image {}",
                    layer_index,
                    image_tiles.len(),
                    tileset_id,
                    image_index
                );
                let Some(texture_handle) = textures.get(*tileset_id, image_index) else {
                    warn!(
                        "Missing texture for tileset {} image {}",
                        tileset_id, image_index
                    );
                    continue;
                };

                let map_size = TilemapSize {
                    x: level.width,
                    y: level.height,
                };

                let tilemap_tile_size = TilemapTileSize {
                    x: tile_size,
                    y: tile_size,
                };

                let grid_size: TilemapGridSize = tilemap_tile_size.into();
                let mut tile_storage = TileStorage::empty(map_size);
                let tilemap_entity = commands.spawn_empty().id();

                // Spawn tiles for this image
                for (x, y, local_tile_index) in image_tiles {
                    let tile_pos = TilePos { x, y };
                    let tile_entity = commands
                        .spawn(TileBundle {
                            position: tile_pos,
                            tilemap_id: TilemapId(tilemap_entity),
                            texture_index: TileTextureIndex(local_tile_index),
                            ..default()
                        })
                        .id();
                    tile_storage.set(&tile_pos, tile_entity);
                }

                // Z-offset: layer_index * 0.1 + image_index * 0.01
                // This ensures proper ordering: all images in layer 0 render before layer 1
                let layer_z = layer_index as f32 * 0.1 + image_index as f32 * 0.01;

                commands.entity(tilemap_entity).insert((
                    TilemapBundle {
                        grid_size,
                        map_type: TilemapType::Square,
                        size: map_size,
                        storage: tile_storage,
                        texture: TilemapTexture::Single(texture_handle.clone()),
                        tile_size: tilemap_tile_size,
                        transform: Transform::from_xyz(0.0, 0.0, layer_z),
                        ..default()
                    },
                    MapLayerIndex(layer_index),
                ));

                commands.entity(map_entity).add_child(tilemap_entity);
            }
        } else {
            info!(
                "  Layer {} is not a tile layer (entity layer or other)",
                layer_index
            );
        }
    }

    // Spawn entities if registry is provided
    if let Some(registry) = entity_registry {
        registry.spawn_all(commands, &level.entities, transform);
    }

    map_entity
}
