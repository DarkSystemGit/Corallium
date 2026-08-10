//! Tracks keyboard state across frames: which keys are currently held down,
//! and which were pressed/released since the last `update()` call.

use std::collections::HashSet;
use winit::event::ElementState;
use winit::keyboard::KeyCode;

/// A keyboard key. This re-exports winit's physical key codes so callers
/// don't need to depend on winit directly.
pub type Key = KeyCode;

/// Tracks held keys plus edge-triggered press/release events for a single
/// frame. Call [`InputState::begin_frame`] once per `Window::update()` to
/// clear the per-frame edge sets before pumping new events into it.
#[derive(Default, Debug)]
pub struct InputState {
    held: HashSet<Key>,
    pressed_this_frame: HashSet<Key>,
    released_this_frame: HashSet<Key>,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears the per-frame edge-triggered sets. Call at the start of each
    /// `update()`, before pumping OS events.
    pub fn begin_frame(&mut self) {
        self.pressed_this_frame.clear();
        self.released_this_frame.clear();
    }

    /// Feed a raw key event from winit into the tracker.
    pub fn process_key_event(&mut self, key: Key, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.held.insert(key) {
                    self.pressed_this_frame.insert(key);
                }
            }
            ElementState::Released => {
                self.held.remove(&key);
                self.released_this_frame.insert(key);
            }
        }
    }

    /// True if `key` is currently held down.
    pub fn is_key_down(&self, key: Key) -> bool {
        self.held.contains(&key)
    }

    /// True if `key` transitioned from up to down during the most recent
    /// `update()` call (edge-triggered; won't repeat while held).
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.pressed_this_frame.contains(&key)
    }

    /// True if `key` transitioned from down to up during the most recent
    /// `update()` call.
    pub fn is_key_released(&self, key: Key) -> bool {
        self.released_this_frame.contains(&key)
    }

    /// All keys currently held down.
    pub fn held_keys(&self) -> impl Iterator<Item = &Key> {
        self.held.iter()
    }

    /// All keys currently held down, as an owned `Vec` (minifb-style
    /// `get_keys()`). Prefer [`InputState::held_keys`] to avoid the
    /// allocation if you're just iterating.
    pub fn get_keys(&self) -> Vec<Key> {
        self.held.iter().copied().collect()
    }

    /// Clears all state. Useful when focus is lost, to avoid "stuck keys".
    pub fn clear(&mut self) {
        self.held.clear();
        self.pressed_this_frame.clear();
        self.released_this_frame.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_then_hold_then_release() {
        let mut input = InputState::new();

        input.begin_frame();
        input.process_key_event(KeyCode::Space, ElementState::Pressed);
        assert!(input.is_key_down(KeyCode::Space));
        assert!(input.is_key_pressed(KeyCode::Space));

        // Next frame: still held, but no longer an edge-triggered "pressed".
        input.begin_frame();
        assert!(input.is_key_down(KeyCode::Space));
        assert!(!input.is_key_pressed(KeyCode::Space));

        // OS may repeat "Pressed" events while held (key repeat) -- must not
        // re-trigger is_key_pressed.
        input.process_key_event(KeyCode::Space, ElementState::Pressed);
        assert!(!input.is_key_pressed(KeyCode::Space));

        input.begin_frame();
        input.process_key_event(KeyCode::Space, ElementState::Released);
        assert!(!input.is_key_down(KeyCode::Space));
        assert!(input.is_key_released(KeyCode::Space));
    }

    #[test]
    fn clear_resets_everything() {
        let mut input = InputState::new();
        input.process_key_event(KeyCode::KeyA, ElementState::Pressed);
        input.clear();
        assert!(!input.is_key_down(KeyCode::KeyA));
    }

    #[test]
    fn get_keys_returns_all_held() {
        let mut input = InputState::new();
        input.process_key_event(KeyCode::KeyW, ElementState::Pressed);
        input.process_key_event(KeyCode::Space, ElementState::Pressed);
        let mut keys = input.get_keys();
        keys.sort_by_key(|k| format!("{k:?}"));
        let mut expected = vec![KeyCode::KeyW, KeyCode::Space];
        expected.sort_by_key(|k| format!("{k:?}"));
        assert_eq!(keys, expected);

        input.process_key_event(KeyCode::Space, ElementState::Released);
        assert_eq!(input.get_keys(), vec![KeyCode::KeyW]);
    }
}
