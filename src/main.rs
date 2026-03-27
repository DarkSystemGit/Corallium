mod compiler;
mod devices;
mod executable;
mod test;
mod util;
mod vm;
use test::run_cases;
fn main() {
    run_cases();
}
