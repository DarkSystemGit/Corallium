fn read(section: i16, addr: i32, len: i32, dest: &void) -> void {}
fn write(section: i16, addr: i32, len: i32, buffer: &void) -> void {}
fn loadSectors(start: i16, count: i16, dest: i32) -> void {}
fn linkedFileStart() -> i16 {}
fn sectorCount()-> i16 {}
