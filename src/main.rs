mod compiler;
mod devices;
mod executable;
mod test;
mod util;
mod vm;

use compiler::lexer::Lexer;
fn compiler() {
    use compiler::backend::Backend;

    let src = r#"import "std";
enum Color {
    Red,
    Green,
    Blue
}
struct Person {
    name: [char; 4],
    age: u32,
    color: Color
}
union Shape{
    circle: [f32;3],
    rectangle: [f32; 4]
}
type Man=Person;
type Number = i32;
fn mock_print_a()->void{}
fn mock_print_b()->void{}
fn mock_print_c()->void{}
fn main() -> Person {

let ix: i32 = 5;
let x: Number = ix as Number;
if(!((x as i16)+ 3-9 == 0) & (x as bool)==false){
    mock_print_a();
}
//hi im a comment
let y: [Number; 3] = [1,2,3];
let z: &Number = &x;
let w: [Number; 10] = [1,2,3,4,5,6,7,8,9,10];
let bob: Person = Man {
    name: "Bob",
    age: 30 as u32,
    color: Color::Red
};
let a: i32=w[1];
let shape: Shape = Shape::circle([1.0, 2.0,3.0]);
let shape_size: u32 = sizeof(Shape);
match shape {
    Shape::circle([x, y,_]) -> {
        mock_print_b();
    },
    Shape::rectangle([x1, y1, x2, y2]) -> {
        mock_print_c();
    },
};
let bobref: &Person = &bob;
return *bobref;
}
        "#;
    let mut compiler = Backend::new(src, "test.micro");
    compiler.select_instructions();
    println!("{:?}", compiler.functions);

    //println!("{:?}", lexer.lex());
}
fn main() {
    compiler();
}
