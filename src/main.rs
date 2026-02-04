mod compiler;
mod devices;
mod executable;
mod test;
mod util;
mod vm;
use compiler::parser::Parser;
use test::run_cases;
fn main() {
    let src = r#"
import "std";
enum Color {
    Red,
    Green,
    Blue
}
struct Person {
    name: [char; 32],
    age: u32,
    color: Color
}
union Shape{
    circle: [f32;2],
    rectangle: [f32; 4]
}
fn main() -> Person {
println("Hello, world!");
let x: i32 = 5;
if(!(x + 3 -9 == 0) & x>0){
    println("x is positive");
}
let y: [&i32; 10] = [0, 10,11,12,0,0,0,0,0,0,0];
let z: &i32 = &x;
let w: &[i32; 10] = [0,0,0,0,0,0,0,0];
let bob: Person = Person {
    name: "Bob",
    age: 30,
    color: Color::Red
};
let shape: Shape = Shape::Circle([1.0, 2.0]);
let bobref: &Person = &bob;
return *bobref;
}
        "#
    .to_string();
    let mut parser = Parser::new(src, "src/test.micro".to_string());
    let ast = parser.parse();
    println!("{:?}", ast);
}
