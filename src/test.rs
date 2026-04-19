use crate::compiler::compile;
use crate::devices::audio::load_wav;
use crate::devices::disk::{Disk, DiskSection, DiskSectionType};
use crate::executable::{Bytecode, Data, Executable, Fn, Library};
use crate::util::{convert_float, convert_u32_to_i16, flatten_vec, gen_3d_matrix};
use crate::vm::CommandType::*;
use crate::vm::CommandType::{Load, Mov, NOP};
use crate::vm::{DataType, Machine};
use prompted::input;
use std::{fs, vec};
struct TestCase {
    name: String,
    ttype: TestType,
}
enum TestType {
    External(Executable),
    Internal(fn(&mut Machine)),
    Compiler(String),
}
impl TestCase {
    fn new(name: &str, ttype: TestType) -> Self {
        TestCase {
            name: name.to_string(),
            ttype,
        }
    }
}
pub fn run_cases() {
    let debug = input!("Debug? [y/n]") == "y";
    for case in get_cases() {
        println!("Executing test {}", case.name);
        let mut machine = Machine::new(debug);
        match case.ttype {
            TestType::External(exe) => {
                let mut disk: Disk = vec![DiskSection {
                    section_type: DiskSectionType::Entrypoint,
                    id: 0,
                    data: vec![],
                }] as Disk;
                exe.build(0, &mut disk, debug);
                machine.set_disk(disk);
                machine.run();
            }
            TestType::Internal(ref func) => func(&mut machine),
            TestType::Compiler(code) => {
                let (exe, logs) = compile(&case.name, code.as_str());
                let mut disk: Disk = vec![DiskSection {
                    section_type: DiskSectionType::Entrypoint,
                    id: 0,
                    data: vec![],
                }] as Disk;
                exe.build(0, &mut disk, debug);
                machine.set_disk(disk);
                if debug {
                    for log in logs {
                        println!("{}", log);
                    }
                }
                machine.run();
            }
        }
        println!("Final State:");
        machine.dump_state();
        input!("Press Enter to continue...");
    }
}
fn get_cases() -> Vec<TestCase> {
    let cases = vec![
        comp_struct_ret(),
        comp_optional(),
        comp_arithmentic(),
        comp_loop(),
        comp_control_flow(),
        comp_data_structs(),
        comp_func(),
        TestCase::new("stack_case", TestType::Internal(stack_case)),
        gfx_case(),
        orig_case(),
    ];
    cases
        .into_iter()
        .filter(|x| input!("Run case {} [y/n]:", x.name) == "y")
        .collect()
}
//working features
// var creation&assign
// arithmetic&casting
// loops
// defer
// calls&args
//if
// match
// structs! & unions &Enums
// optional!
fn comp_optional() -> TestCase {
    TestCase::new(
        "CompilerOptionalType",
        TestType::Compiler(
            r#"
                fn optionalb(x:i32)->bool?{
                    return match x%(3 as i32){
                        1->Some(true),
                        _->None
                    };
                }
                fn optional(x: i32)->bool?{
                    return match x%(2 as i32){
                        1->Some(true),
                        _->Some(try optionalb(x))
                    };
                }
                fn main()->void{
                    let testA: bool=try optional(5 as i32);
                    let testB: bool=try optional(6 as i32) catch{
                        false
                    };
                    testA;
                    testB;
                    return;
                }
            "#
            .to_string(),
        ),
    )
}
fn comp_data_structs() -> TestCase {
    TestCase::new(
        "CompilerDataStructures",
        TestType::Compiler(
            r#"
                union Shape{
                    rectangle: [u32;2],
                    circle: u32,
                    triangle: [u32;3]
                }
                enum Color{
                    Red,
                    Green,
                    Blue
                }
                struct Person{
                    name: [char; 4],
                    favoriteColor: Color,
                    favoriteShape: Shape,
                    age: u32
                }
                fn main()->void{
                    let bob: Person=Person{
                        name: "Bob",
                        age: 20 as u32,
                        favoriteColor: Color::Blue,
                        favoriteShape: Shape::triangle([3,4,5])
                    };
                    let name: [char; 4]=bob.name;
                    let color: i16=match bob.favoriteColor{
                        Color::Blue->2,
                        Color::Red->1,
                        _->99
                    };
                    let age: u32=bob.age;
                    let isTriangle: i16=match bob.favoriteShape{
                        Shape::triangle(_)->11,
                        _->99
                    };
                    age;
                    return;
                }
                "#
            .to_string(),
        ),
    )
}
fn comp_control_flow() -> TestCase {
    TestCase::new(
        "CompilerControlFlow",
        TestType::Compiler(
            r#"
                fn main() -> void {
                    let x: i16=1+2+3;
                    let y: i16=4*5*6;
                    let z: i32=(y as i32)/(x as i32);
                    let a: i16=0;
                    if((z as i16)<25){
                        for(let i: i16=0;i<10;i=i+1){
                            if (i==8){
                                break;
                            };
                            match i%2{
                                0->a=a+1,
                                _->continue
                            };
                        };
                    }else{
                        a=-1;
                    };
                    a;
                    return;
                }
                "#
            .to_string(),
        ),
    )
}
fn comp_arithmentic() -> TestCase {
    TestCase::new(
        "CompilerArithmetic",
        TestType::Compiler(
            r#"
                fn main() -> void {
                    let x: i16=1+2+3; // 67
                    let y: i16=4*5*6;
                    let z: i32=(y as i32)/(x as i32);
                    return;
                }
                "#
            .to_string(),
        ),
    )
}
fn comp_loop() -> TestCase {
    TestCase::new(
        "CompilerLoop",
        TestType::Compiler(
            r#"
                fn main() -> void {
                    let acc: i32=0;
                    defer acc=acc+(1 as i32);
                    while (acc<(50 as i32)){
                        defer acc=acc+add;
                        let add: i32=(5 as i32);
                    };
                    return;
                }
                "#
            .to_string(),
        ),
    )
}
fn comp_func() -> TestCase {
    TestCase::new(
        "CompilerFunc",
        TestType::Compiler(
            r#"
                fn body()->i32{
                    let add: i32=(5 as i32);
                    return add;
                }
                fn forloop(start: i32)->i32{
                let acc: i32=start;
                defer acc=acc+(1 as i32);
                while (acc<(50 as i32)){
                    defer acc=acc+add;
                    let add: i32=body();
                };
                return acc;
                }

                fn main() -> void {
                    let c:i32=0;
                    for (let i: i16=0;i<10;i=i+1){
                        c=c+forloop(i as i32);
                    };
                    c;
                    return;
                }

                "#
            .to_string(),
        ),
    )
}
fn comp_struct_ret() -> TestCase {
    TestCase::new(
        "CompilerStructReturn",
        TestType::Compiler(
            r#"
                struct Point{
                    x:i16,
                    y:i16
                }
                fn makePoint(x: i16,y:i16)->Point{
                    return Point {x: x, y:y};
                }
                fn main() -> void {
                    let p: Point=makePoint(10,20);
                    p.x;
                    p.y;
                    return;
                }

                "#
            .to_string(),
        ),
    )
}
fn stack_case(machine: &mut Machine) {
    machine
        .core
        .stack
        .push(DataType::Int(42), &mut machine.core.srp);
    machine
        .core
        .stack
        .push(DataType::Int32(67), &mut machine.core.srp);
    machine
        .core
        .stack
        .push(DataType::Float(1024.0), &mut machine.core.srp);
    let mut byte = Vec::new();
    for i in 0..5 {
        byte.push(machine.memory.read(16 * 1024 * 1024 + i, machine));
    }
    dbg!(&byte);
    machine.memory.write_range(
        16 * 1024 * 1024..16 * 1024 * 1024 + 2,
        vec![1, 2],
        &mut machine.core,
    );
    machine.memory.write_range(
        16 * 1024 * 1024 + 3..16 * 1024 * 1024 + 5,
        convert_float(96.0),
        &mut machine.core,
    );
    byte.clear();
    let mut byte = Vec::new();
    for i in 0..5 {
        byte.push(machine.memory.read(16 * 1024 * 1024 + i, machine));
    }
    dbg!(&byte);
}
fn gfx_case() -> TestCase {
    let mut exe = Executable::new();
    let mut main_fn = Fn::new("main".to_string(), vec![]);
    let atlas = exe.add_constant(vec![
        Data::Int(3),
        Data::Bytes(vec![0; 2 * 64]), //transparency
        Data::Bytes(flatten_vec(vec![convert_u32_to_i16(0xFFFFFFFF); 64])),
        Data::Bytes(flatten_vec(vec![convert_u32_to_i16(0xAABBCCFF); 64])),
    ]);
    let layer_data = exe.add_constant(vec![Data::Bytes(vec![1; 30 * 40])]);
    let matrix = gen_3d_matrix(0.0, 0.0, 10.0, 160.0, 0.0, 1.0, 240);
    let packaged_matrix = matrix
        .0
        .iter()
        .map(|x| x.map(|y| y.map(|z| Data::Float(z))))
        .flatten()
        .flatten()
        .collect::<Vec<Data>>();
    let loc = matrix
        .1
        .iter()
        .map(|x| Data::Int32(*x))
        .collect::<Vec<Data>>();
    let layer_transform = exe.add_constant(packaged_matrix);
    let layer_loc = exe.add_constant(loc);
    let layer_transform_opt = exe.add_constant(vec![Data::ConstantLoc(layer_transform)]);
    let layer_loc_opt = exe.add_constant(vec![Data::ConstantLoc(layer_loc)]);
    let layer = exe.add_constant(vec![
        Data::Bytes(vec![0, 0, 0, 30, 40]),
        Data::ConstantLoc(layer_data),
        Data::Bytes(vec![2]),
        Data::ConstantLoc(layer_transform_opt),
        Data::ConstantLoc(layer_loc_opt),
    ]);
    let sprite_data = exe.add_constant(vec![Data::Bytes(vec![2; 4 * 4])]);
    let sprite = exe.add_constant(vec![
        Data::Bytes(vec![0, 0, 0, 1, 4, 4]),
        Data::ConstantLoc(sprite_data),
    ]);
    main_fn.add_symbol("controls", 12);
    let update_block = main_fn.add_block(
        vec![
            Bytecode::Command(IO),
            Bytecode::Int(3),
            Bytecode::Int(3),
            Bytecode::Command(AddEx),
            Bytecode::ConstantLoc(sprite),
            Bytecode::Int(1),
            Bytecode::Command(Load),
            Bytecode::Register(EX1),
            Bytecode::Register(R1),
            Bytecode::Command(Add),
            Bytecode::Register(R1),
            Bytecode::Int(1),
            Bytecode::Command(Store),
            Bytecode::Register(EX1),
            Bytecode::Register(R1),
            Bytecode::Command(AddEx),
            Bytecode::Register(ARP),
            Bytecode::Symbol("controls".to_string(), 0),
            Bytecode::Command(Push),
            Bytecode::Register(EX1),
            Bytecode::Command(IO),
            Bytecode::Int(3),
            Bytecode::Int(4),
            Bytecode::Command(Jump),
            Bytecode::BlockLoc(-1),
        ],
        false,
    );
    main_fn.add_block(
        vec![
            Bytecode::Command(Push),
            Bytecode::ConstantLoc(atlas),
            Bytecode::Command(IO),
            Bytecode::Int(3),
            Bytecode::Int(0),
            Bytecode::Command(Push),
            Bytecode::ConstantLoc(layer),
            Bytecode::Command(IO),
            Bytecode::Int(3),
            Bytecode::Int(1),
            Bytecode::Command(Push),
            Bytecode::ConstantLoc(sprite),
            Bytecode::Command(IO),
            Bytecode::Int(3),
            Bytecode::Int(2),
            Bytecode::Command(Jump),
            Bytecode::BlockLoc(update_block),
        ],
        true,
    );
    exe.add_fn(main_fn);
    TestCase {
        name: "gfx".to_string(),
        ttype: TestType::External(exe),
    }
}
fn orig_case() -> TestCase {
    let mut main_fn = Fn::new("main".to_string(), vec![]);
    let mut exe = Executable::new();
    let constant = exe.add_constant(vec![Data::Bytes(vec![-5, 0])]);
    let sound_file: Vec<i16> = load_wav(fs::read("sample.wav").unwrap().as_slice())
        .iter()
        .flat_map(|x| convert_float(*x))
        .collect();
    let file_size = sound_file.len() as i32;
    let mut another_fn = Fn::new("another_fn".to_string(), vec![]);
    let another_constant = exe.add_constant(vec![Data::Bytes(vec![1, 2])]);
    another_fn.add_block(
        vec![
            Bytecode::Command(Load),
            Bytecode::ConstantLoc(another_constant),
            Bytecode::Register(R2),
            Bytecode::Command(Add),
            Bytecode::ConstantLoc(another_constant),
            Bytecode::Int(1),
            Bytecode::Command(Load),
            Bytecode::Register(R1),
            Bytecode::Register(R1),
            Bytecode::Command(Add),
            Bytecode::Register(R1),
            Bytecode::Register(R2),
            Bytecode::Command(Push),
            Bytecode::Register(R1),
            Bytecode::Command(Return),
            Bytecode::Int(1),
            Bytecode::SymbolSectionLen(),
            Bytecode::ArgCount(),
        ],
        true,
    );
    exe.add_fn(another_fn);
    let mut symbol_lib = Library::new("symbolLib".to_string());
    let mut symbolfn = Fn::new("symbol".to_string(), vec![1]);
    symbolfn.add_symbol("testsymbol", 2);
    symbolfn.add_block(
        vec![
            Bytecode::Command(AddEx),
            Bytecode::Register(ARP),
            Bytecode::Argument(0),
            Bytecode::Command(Load), //stack gets compressed into i16
            Bytecode::Register(EX1),
            Bytecode::Register(EX1),
            Bytecode::Command(NOP),
            Bytecode::Command(AddEx),
            Bytecode::Symbol("testsymbol".to_string(), 0),
            Bytecode::Register(ARP),
            Bytecode::Command(Store),
            Bytecode::Register(EX1),
            Bytecode::Int32(4096),
            Bytecode::Command(Load),
            Bytecode::Register(EX1),
            Bytecode::Register(EX1),
            Bytecode::Command(AddEx),
            Bytecode::Register(EX1),
            Bytecode::Int(1),
            Bytecode::Command(Push),
            Bytecode::Register(EX1),
            Bytecode::Command(Return),
            Bytecode::Int(1),
            Bytecode::SymbolSectionLen(),
            Bytecode::ArgCount(),
        ],
        true,
    );
    symbol_lib.add_fn(symbolfn);
    let do_nothing =
        main_fn.add_block(vec![Bytecode::Command(Jump), Bytecode::BlockLoc(-1)], false);
    main_fn.add_block(
        vec![
            Bytecode::Command(Call),
            Bytecode::FunctionRef("another_fn".to_string()),
            Bytecode::Command(Pop),
            Bytecode::Register(R1),
            Bytecode::Command(Mov),
            Bytecode::Register(R1),
            Bytecode::Register(F1),
            Bytecode::Command(Addf),
            Bytecode::Float(0.5),
            Bytecode::Register(F1),
            Bytecode::Command(Store),
            Bytecode::ConstantLoc(constant),
            Bytecode::Register(F1),
            Bytecode::Command(Loadf),
            Bytecode::ConstantLoc(constant),
            Bytecode::Register(F1),
            Bytecode::Command(Push),
            Bytecode::Int(25),
            Bytecode::Command(Call),
            Bytecode::FunctionRef("testLib::symbolLib::symbol".to_string()), //breaks ARP
            Bytecode::Command(Pop),
            Bytecode::Register(R1),
            Bytecode::Int(0),
            Bytecode::Command(IO),
            Bytecode::Int(2),
            Bytecode::Int(0),
            Bytecode::Command(Call),
            Bytecode::FunctionRef("testLib::main".to_string()),
            Bytecode::Command(Jump),
            Bytecode::BlockLoc(do_nothing),
        ],
        true,
    );
    exe.add_fn(main_fn);
    let mut test_lib = Library::new("testLib".to_string());
    test_lib.add_constant(vec![Data::Bytes(vec![6, 7])]);
    let sound_sample = test_lib.add_constant(vec![Data::Bytes(sound_file)]);
    test_lib.add_fn(Fn::new_with_blocks(
        "main".to_string(),
        vec![],
        vec![vec![
            Bytecode::Command(NOP),
            Bytecode::Command(PushEx),
            Bytecode::Int32(file_size),
            Bytecode::Command(PushEx),
            Bytecode::ConstantLoc(sound_sample),
            Bytecode::Command(Push),
            Bytecode::Int(9),
            Bytecode::Command(IO),
            Bytecode::Int(1),
            Bytecode::Int(6),
            Bytecode::Command(Return),
            Bytecode::Int(0),
            Bytecode::SymbolSectionLen(),
            Bytecode::ArgCount(),
        ]],
    ));
    symbol_lib.link_lib(&mut test_lib);
    test_lib.link(&mut exe);
    TestCase {
        name: "orig".to_string(),
        ttype: TestType::External(exe),
    }
}
