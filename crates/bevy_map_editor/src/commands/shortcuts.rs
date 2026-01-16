//! Keyboard shortcut handling

use bevy::prelude::*;

use crate::ui::PendingAction;
use crate::{EditorState, EditorViewMode};

/// Handle keyboard shortcuts
pub fn handle_keyboard_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut editor_state: ResMut<EditorState>,
) {
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if ctrl {
        // Ctrl+Z - Undo
        if keyboard.just_pressed(KeyCode::KeyZ) && !shift {
            editor_state.pending_action = Some(PendingAction::Undo);
        }
        // Ctrl+Shift+Z or Ctrl+Y - Redo
        if (keyboard.just_pressed(KeyCode::KeyZ) && shift) || keyboard.just_pressed(KeyCode::KeyY) {
            editor_state.pending_action = Some(PendingAction::Redo);
        }
        // Ctrl+C - Copy
        if keyboard.just_pressed(KeyCode::KeyC) {
            editor_state.pending_action = Some(PendingAction::Copy);
        }
        // Ctrl+X - Cut
        if keyboard.just_pressed(KeyCode::KeyX) {
            editor_state.pending_action = Some(PendingAction::Cut);
        }
        // Ctrl+V - Paste
        if keyboard.just_pressed(KeyCode::KeyV) {
            editor_state.pending_action = Some(PendingAction::Paste);
        }
        // Ctrl+A - Select All
        if keyboard.just_pressed(KeyCode::KeyA) {
            editor_state.pending_action = Some(PendingAction::SelectAll);
        }
        // Ctrl+Shift+S - Create Stamp from Selection
        if keyboard.just_pressed(KeyCode::KeyS) && shift {
            editor_state.pending_action = Some(PendingAction::CreateStampFromSelection);
        }
        // Ctrl+S - Save
        if keyboard.just_pressed(KeyCode::KeyS) {
            editor_state.pending_action = Some(PendingAction::Save);
        }
        // Ctrl+O - Open
        if keyboard.just_pressed(KeyCode::KeyO) {
            editor_state.pending_action = Some(PendingAction::Open);
        }
        // Ctrl+N - New
        if keyboard.just_pressed(KeyCode::KeyN) {
            editor_state.pending_action = Some(PendingAction::New);
        }
    }

    // Delete key
    if keyboard.just_pressed(KeyCode::Delete) || keyboard.just_pressed(KeyCode::Backspace) {
        if !editor_state.tile_selection.is_empty() {
            editor_state.pending_delete_selection = true;
        }
    }

    // Escape key - cancel move operation, paste mode, or clear selection (in priority order)
    if keyboard.just_pressed(KeyCode::Escape) {
        if editor_state.is_moving {
            // Signal to cancel the move operation (handled in tools system which has Project access)
            editor_state.pending_cancel_move = true;
        } else if editor_state.is_pasting {
            editor_state.is_pasting = false;
        } else {
            editor_state.tile_selection.clear();
        }
    }

    // Non-Ctrl shortcuts (only when not typing in text fields)
    if !ctrl {
        // W key - toggle World view
        if keyboard.just_pressed(KeyCode::KeyW) {
            editor_state.view_mode = match editor_state.view_mode {
                EditorViewMode::Level => EditorViewMode::World,
                EditorViewMode::World => EditorViewMode::Level,
            };
        }

        // L key - switch to Level view
        if keyboard.just_pressed(KeyCode::KeyL) {
            editor_state.view_mode = EditorViewMode::Level;
        }

        // X key - toggle horizontal flip for painting
        if keyboard.just_pressed(KeyCode::KeyX) {
            editor_state.paint_flip_x = !editor_state.paint_flip_x;
        }

        // Y key - toggle vertical flip for painting
        if keyboard.just_pressed(KeyCode::KeyY) {
            editor_state.paint_flip_y = !editor_state.paint_flip_y;
        }
    }
}
