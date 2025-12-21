# bevy_map_dialogue

Dialogue tree system for the bevy_map_editor ecosystem.

Part of [bevy_map_editor](https://github.com/jbuehler23/bevy_map_editor).

## Features

- Branching dialogue trees
- Multiple node types (Text, Choice, Condition, Action, End)
- Player choices with optional conditions
- Visual node editor in bevy_map_editor
- Event-based dialogue flow

## Node Types

| Type        | Description                                   |
|-------------|-----------------------------------------------|
| `Text`      | NPC speaks, then continues to next node       |
| `Choice`    | Player selects from options                   |
| `Condition` | Branch based on game state                    |
| `Action`    | Trigger game actions (give item, start quest) |
| `End`       | Dialogue terminates                           |

## Types

| Type             | Description                       |
|------------------|-----------------------------------|
| `DialogueTree`   | Complete dialogue with all nodes  |
| `DialogueNode`   | Single node in the tree           |
| `DialogueChoice` | Player choice option              |
| `DialogueRunner` | Resource tracking active dialogue |
| `DialogueHandle` | Component holding dialogue asset  |

## Events

| Event                 | Description          |
|-----------------------|----------------------|
| `StartDialogueEvent`  | Begin a dialogue     |
| `DialogueChoiceEvent` | Player made a choice |
| `DialogueEndEvent`    | Dialogue finished    |

## Usage

### Creating Dialogues (Code)

```rust
use bevy_map::dialogue::{DialogueTree, DialogueNode, DialogueChoice};

let mut tree = DialogueTree::new("merchant_greeting");

// Add a choice node
let mut start = DialogueNode::new_choice("Merchant", "Welcome! What would you like?");
start.choices = vec![
    DialogueChoice::new("Buy items", "shop"),
    DialogueChoice::new("Sell items", "sell"),
    DialogueChoice::new("Goodbye", "end"),
];
tree.add_node(start);
```

### Creating Dialogues (Editor)

The Dialogue Editor panel provides:
1. Visual node graph
2. Drag-and-drop node creation
3. Connection drawing between nodes
4. Property editing in inspector

### Starting a Dialogue

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;

fn interact(
    mut start_events: MessageWriter<StartDialogueEvent>,
    dialogue_assets: Res<Assets<DialogueTree>>,
    query: Query<(Entity, &DialogueHandle)>,
) {
    for (entity, handle) in query.iter() {
        start_events.write(StartDialogueEvent {
            speaker_entity: entity,
            dialogue: handle.0.clone(),
        });
    }
}
```

### Reading Dialogue State

```rust
use bevy_map::prelude::*;

fn show_dialogue_ui(
    runner: Res<DialogueRunner>,
    dialogues: Res<Assets<DialogueTree>>,
) {
    if !runner.is_active() { return; }

    let Some(handle) = &runner.dialogue_handle else { return };
    let Some(tree) = dialogues.get(handle) else { return };
    let Some(node_id) = &runner.current_node_id else { return };
    let Some(node) = tree.get_node(node_id) else { return };

    // Display node.speaker, node.text, node.choices
}
```

### Auto-Loading from Maps

```rust
use bevy_map::runtime::DialogueTreeHandle;

commands.spawn(DialogueTreeHandle::new(
    asset_server.load("maps/game.map.json"),
    "merchant_greeting",
));
```

## Plugin

Add `DialoguePlugin` for dialogue handling:

```rust
use bevy_map::prelude::*;

app.add_plugins(DialoguePlugin);
```

Note: `MapRuntimePlugin` includes this automatically.

## License

MIT OR Apache-2.0
