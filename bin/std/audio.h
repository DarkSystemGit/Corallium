fn pause() -> void {}
fn unpause() -> void {}
fn volume(channel: i16, newVolume: f32) -> void {}
fn pan(channel: i16, left: f32, right: f32) -> void {}
fn frequency(channel: i16, newFrequency: f32) -> void {}
fn masterVolume(newVolume: i32) -> void {}
fn loadSound(channel: i16, sample: [f32], len: i32) -> void {}
fn setLoop(channel: i16, enabled: bool) -> void {}
