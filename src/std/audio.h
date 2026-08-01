fn pause() -> void {}
fn unpause() -> void {}
fn volume(channel: i16, newVolume: f32) -> void {}
fn pan(channel: i16, left: f32, right: f32) -> void {}
fn frequency(channel: i16, newFrequency: f32) -> void {}
fn masterVolume(newVolume: i32) -> void {}
fn loadSound(channel: i16, sample: [f32], len: i32) -> void {}
fn setLoop(channel: i16, enabled: bool) -> void {}
fn schedule(channel: i16, time: i32, commandType: i16, value: f32) -> void {}
fn masterClock() -> i32 {}
fn scheduleWithId(channel: i16, time: i32, commandType: i16, value: f32, scheduleId: i32) -> void {}
fn deschedule(scheduleId: i32) -> void {}
fn nextScheduleId() -> i32 {}
fn scheduleDone(scheduleId: i32) -> i32 {}
