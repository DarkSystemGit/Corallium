use crate::devices::disk::{Disk, DiskSection, DiskSectionType};
use crate::executable::Bytecode::{
    ArgCount, Argument, BlockLoc, Command, ConstantLoc, Float, FunctionRef, HeapStart, Int, Int32,
    Register, SymbolSectionLen,
};
use crate::util::*;
use crate::vm::CommandType;
use crate::vm::CommandType::{Add, IO, Jump, Load, Mov, Push, R1, R2, R3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    name: String,
    fns: Vec<Fn>,
    pub constants: Vec<Vec<Data>>,
}
impl Library {
    pub fn new(name: String) -> Library {
        Library {
            name,
            fns: vec![],
            constants: vec![],
        }
    }

    pub fn add_constant(&mut self, constant: Vec<Data>) -> usize {
        self.constants.push(constant);
        self.constants.len() - 1
    }
    pub fn add_fn(&mut self, mut func: Fn) -> usize {
        func.name = format!("{}::{}", self.name, func.name);
        func.blocks.iter_mut().for_each(|block| {
            for i in block.iter_mut() {
                if let Bytecode::FunctionRef(func_ref) = i {
                    *i = Bytecode::FunctionRef(
                        if func_ref.starts_with(&format!("{}::", self.name)) {
                            func_ref.clone()
                        } else {
                            format!("{}::{}", self.name, func_ref)
                        },
                    );
                }
            }
        });
        self.fns.push(func);
        self.fns.len() - 1
    }
    pub fn link(&self, exe: &mut Executable) {
        let const_offset = exe.constants.data_sec.len();
        for constant in &self.constants {
            exe.add_constant(constant.clone());
        }
        for mut func in self.fns.clone() {
            func.blocks.iter_mut().for_each(|block| {
                for i in block.iter_mut() {
                    if let Bytecode::ConstantLoc(constant) = i {
                        *i = Bytecode::ConstantLoc(*constant + const_offset);
                    }
                }
            });
            exe.add_fn(func);
        }
    }
    pub fn link_lib(&self, lib: &mut Library) {
        let const_off = lib.constants.len();
        for constant in &self.constants {
            lib.add_constant(constant.clone());
        }
        for mut func in self.fns.clone() {
            func.blocks.iter_mut().for_each(|block| {
                for i in block.iter_mut() {
                    if let Bytecode::ConstantLoc(constant) = i {
                        *i = Bytecode::ConstantLoc(*constant + const_off);
                    }
                }
            });
            lib.add_fn(func);
        }
    }
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let encoded = bincode::serialize(self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
        fs::write(path, encoded)
    }
    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let bytes = fs::read(path)?;
        bincode::deserialize(&bytes)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Symbol {
    name: String,
    size: usize,
}
impl Symbol {
    fn new(name: String, size: usize) -> Symbol {
        Symbol { name, size }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SymbolTable {
    symbols: Vec<Symbol>,
}
impl SymbolTable {
    pub fn new() -> SymbolTable {
        SymbolTable {
            symbols: Vec::new(),
        }
    }
    pub fn add_symbol(&mut self, symbol: Symbol) {
        self.symbols.push(symbol);
    }
    pub fn get_symbol(&self, name: &str) -> usize {
        let mut c = 0;
        for symbol in &self.symbols {
            if symbol.name == name {
                return c;
            }
            c += symbol.size;
        }
        panic!("Symbol {} not found", name);
    }
    pub fn len(&self) -> usize {
        let mut c = 0;
        for symbol in &self.symbols {
            c += symbol.size;
        }
        c
    }
    pub fn setup_stack(&self) -> Vec<i16> {
        flatten_vec(vec![
            vec![pack_command(CommandType::AddEx)],
            pack_register(CommandType::SP),
            pack_i32(self.len() as i32),
            vec![pack_command(Mov)],
            pack_register(CommandType::EX1),
            pack_register(CommandType::SP),
            vec![pack_command(Mov)],
            pack_register(CommandType::EX1),
            pack_register(CommandType::SRP),
        ])
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Executable {
    constants: ConstantTable,
    fns: Vec<Fn>,
    loader: Vec<i16>,
    max_loader_len: i16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Bytecode {
    Command(CommandType),
    Register(CommandType),
    Float(f32),
    Int(i16),
    FunctionRef(String),
    ConstantLoc(usize),
    BlockLoc(isize),
    Int32(i32),
    Symbol(String, i32),
    SymbolSectionLen(),
    Argument(usize),
    ArgCount(),
    HeapStart(),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Data {
    Bytes(Vec<i16>),
    Float(f32),
    Int(i16),
    Int32(i32),
    ConstantLoc(usize),
}
fn get_data_len(data: &Data) -> usize {
    match data {
        Data::Bytes(b) => b.len(),
        Data::Float(_f) => 2,
        Data::Int(_i) => 1,
        Data::Int32(_i) => 2,
        Data::ConstantLoc(_c) => 2,
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConstantTable {
    data_sec: Vec<Vec<Data>>,
}
impl ConstantTable {
    fn new() -> ConstantTable {
        ConstantTable { data_sec: vec![] }
    }
    fn add_constant(&mut self, constant: Vec<Data>) -> usize {
        self.data_sec.push(constant);
        self.data_sec.len() - 1
    }
    fn get_constant_offset(&self, id: usize) -> usize {
        let mut offset = 0;
        for (i, constant) in self.data_sec.iter().enumerate() {
            if i == id {
                return offset;
            }
            offset += constant.iter().map(|x| get_data_len(x)).sum::<usize>();
        }
        return 0;
    }
    fn len(&self) -> usize {
        self.data_sec
            .iter()
            .map(|x| x.iter().map(|y| get_data_len(y)).sum::<usize>())
            .sum::<usize>()
    }
    fn serialize(&self, base: usize) -> Vec<i16> {
        self.data_sec
            .concat()
            .iter()
            .map(|x| match x {
                Data::Bytes(b) => b.clone(),
                Data::ConstantLoc(c) => {
                    convert_i32_to_i16((self.get_constant_offset(*c) + base) as i32).to_vec()
                }
                Data::Float(f) => convert_float(*f),
                Data::Int32(i) => convert_i32_to_i16(*i).to_vec(),
                Data::Int(i) => vec![*i],
            })
            .flatten()
            .collect::<Vec<i16>>()
    }
}
//Bytecode Executable Structure
//-mem offset
//-base sector
//-bytecode len
//-bytecode sector count
//-data len
//-data sector count
//bytecode
//data
impl Executable {
    pub(crate) fn new() -> Executable {
        Executable {
            constants: ConstantTable::new(),
            fns: Vec::new(),
            //loader loads from base sector to bytecode sector count, taking only bytecode len%i32::MAX for th final sector.
            //Then, it loads from bytecode sector count+1 to bytecode sector count+data_sector count, loading only data len%i32::MAX for the final sector
            //All of this is loaded at mem offset
            //Pseudocode
            //let next_mem=exec[0];
            //for i in exec[1]..exec[1]+exec[3]+exec[5]-1{
            //  if i==exec[1]+(exec[3]-1){
            //      let fcount=exec[2]%i32::MAX
            //      load(i,fcount,next_mem);
            //      next_mem+=fcount
            //  }else if i==exec[1]+exec[3]+exec[5]-1{
            //      let dfcount=exec[4]%i32::MAX;
            //      load(i,dfcount,next_mem);
            //      next_mem+=dfcount;
            //  }else{
            //      load(i,i32::MAX,next_mem)
            //      next_mem+=i32::MAX
            //  }
            //}
            loader: Self::default_loader(512, 6),
            max_loader_len: 512,
        }
    }
    pub(crate) fn to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let encoded = bincode::serialize(self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
        fs::write(path, encoded)
    }
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let bytes = fs::read(path)?;
        bincode::deserialize(&bytes)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    }
    fn default_loader(max_loader_len: i16, header_len: i16) -> Vec<i16> {
        let mut f = Fn::new("loader".to_string(), vec![]);
        f.symbol_enabled = false;
        f.add_block(
            vec![
                Command(Push),
                Int(0), //dest
                Command(Push),
                Int(1), //count
                Command(Push),
                Int(0), //start sector,
                Command(IO),
                Int(0),
                Int(2), //loadSectors,
                Command(Load),
                Int(max_loader_len + 3),
                Register(R1), //exec::bytecode_sector_count,
                Command(Load),
                Int(max_loader_len + 1),
                Register(R2), //exec::base_sector
                Command(Load),
                Int(max_loader_len + 5),
                Register(R3), //exec::data_sector_count
                Command(Add),
                Register(R1),
                Register(R3), //total sectors
                Command(Push),
                Int(0), //dest
                Command(Push),
                Register(R1), //count
                Command(Push),
                Register(R2), //start sector
                Command(IO),
                Int(0),
                Int(2), //loadSectors
                Command(Jump),
                Int(max_loader_len + header_len),
            ],
            true,
        );
        f.build(0, &HashMap::new(), 0, &ConstantTable::new(), false)
    }
    fn set_loader(&mut self, loader: Vec<i16>) {
        if loader.len() > self.max_loader_len as usize {
            println!("Oversized executable loader");
        }
        self.loader = loader;
    }
    pub(crate) fn add_constant(&mut self, constant: Vec<Data>) -> usize {
        self.constants.add_constant(constant)
    }
    pub(crate) fn add_fn(&mut self, mut data: Fn) -> usize {
        let id = self.fns.len();
        data.id = id;
        self.fns.push(data);
        0
    }
    pub(crate) fn build(mut self, mut offset: usize, disk: &mut Disk, debug: bool) {
        let mut bytecode: Vec<i16> = vec![];
        let mut fn_map: HashMap<String, usize> = HashMap::new();
        let header_len = 6;
        let insertion_jump_len = 5;
        //loader
        offset += self.max_loader_len as usize - 1;
        //headers
        offset += header_len + insertion_jump_len;
        let mut main_loc = 0;
        let data_sec = self.fns.iter_mut().fold(offset + 1, |acc, func| {
            if func.name == "main" {
                main_loc = acc;
            }
            fn_map.insert(func.name.clone(), acc);
            acc + func.len()
        }) as usize;
        //TODO: handle contant building
        for func in self.fns.iter_mut() {
            bytecode.extend(func.build(
                fn_map[&func.name],
                &fn_map,
                data_sec,
                &self.constants,
                debug,
            ))
        }

        Self::insert_bytecode_into_disk(
            &self,
            disk,
            bytecode,
            offset,
            main_loc,
            header_len + insertion_jump_len,
            debug,
            self.constants.serialize(data_sec),
        );
    }
    fn print_structure(
        &self,
        bytecode: &Vec<i16>,
        offset: usize,
        header_len: usize,
        code_sectors: usize,
        data_sectors: usize,
        entrypoint: usize,
    ) {
        println!("-------Header-------");
        println!("Offset: {}", offset - header_len);
        println!(
            "Base Sector: {}",
            ((offset - header_len) as f32 / i32::MAX as f32).floor() as usize
        );
        println!("Bytecode Len: {}", bytecode.len() + 2);
        println!("Code Sector Count: {}", code_sectors);
        println!("Data Len: {}", self.constants.len());
        println!("Data Sector Count: {}", data_sectors);
        println!("-------Insertion Jump-------");
        println!("Jump to Entry Point: %{}", entrypoint);
        println!(
            "-------Functions-------\n{}",
            self.fns
                .iter()
                .map(|x| x.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        );
        println!("-------Bytecode-------");
        display_bytecode(bytecode, 0);
        println!("-------Data-------");
        for (id, data) in self.constants.data_sec.iter().enumerate() {
            for (i, chunk) in data.chunks(32).map(|slice| slice.to_vec()).enumerate() {
                println!(
                    "Constant {:07}: {:?}",
                    i * 32 + self.constants.get_constant_offset(id) + offset + bytecode.len(),
                    chunk
                );
            }
        }
    }

    fn insert_bytecode_into_disk(
        &self,
        disk: &mut Disk,
        bytecode: Vec<i16>,
        mut offset: usize,
        entrypoint: usize,
        header_len: usize,
        debug: bool,
        data: Vec<i16>,
    ) {
        //(total exe code len/max sector data).ceil()
        let code_sectors = ((offset + bytecode.len()) as f32 / i32::MAX as f32).ceil() as usize;
        let data_sectors = (data.len() as f32 / i32::MAX as f32).ceil() as usize;
        if debug {
            self.print_structure(
                &bytecode,
                offset,
                header_len,
                code_sectors,
                data_sectors,
                entrypoint,
            );
        }
        //[mem offset,base sector,bytecode len,bytecode sector count, data len, data sector count]
        let headers = vec![
            offset - header_len,
            ((offset - header_len) as f32 / i32::MAX as f32).floor() as usize,
            bytecode.len() + 2,
            code_sectors,
            data.len(),
            data_sectors,
        ];
        let mut insertion_jump = vec![pack_command(CommandType::Jump)];
        insertion_jump.extend_from_slice(&pack_i32(entrypoint as i32));
        let executable = flatten_vec(vec![
            headers.iter().map(|x| *x as i16).collect(),
            insertion_jump.clone(),
            bytecode,
        ]);
        //remove headers for these calculations
        offset -= header_len;
        let base_sector = (offset as f32 / i32::MAX as f32).floor() as usize;
        let bsector_offset = (offset as f32 % i32::MAX as f32) as usize;
        let data_sector_count = (data.len() as f32 / i32::MAX as f32).ceil() as usize;
        for i in base_sector..code_sectors {
            if i == base_sector {
                let insert_len = match executable.len() < i32::MAX as usize {
                    true => executable.len(),
                    false => i32::MAX as usize,
                };
                resize_vec(bsector_offset + insert_len, &mut disk[i].data, 0);
                disk[i]
                    .data
                    .splice(bsector_offset.., executable[0..insert_len].to_vec());
            } else {
                disk[i].section_type = match disk[base_sector].section_type {
                    DiskSectionType::Entrypoint => DiskSectionType::Code,
                    DiskSectionType::Libary => DiskSectionType::Libary,
                    _ => DiskSectionType::Code,
                };
                let sector_start = (i32::MAX as usize) * (i - base_sector);
                let sector_end = (i32::MAX as usize) * (i - base_sector + 1);
                disk[i].data = executable[sector_start..sector_end].to_vec();
            }
        }

        for i in code_sectors..code_sectors + data_sector_count {
            resize_vec(
                i + 1,
                disk,
                DiskSection {
                    section_type: DiskSectionType::Data,
                    id: -1,
                    data: vec![],
                },
            );
            let iteration = i - code_sectors;
            let data_start = iteration * i32::MAX as usize;
            let data_end = match data.len() < (iteration + 1) * i32::MAX as usize {
                false => (iteration + 1) * i32::MAX as usize,
                true => data.len(),
            };
            disk[i] = DiskSection {
                section_type: DiskSectionType::Data,
                id: i as i16,
                data: data[data_start..data_end].to_vec(),
            };
        }
        let mut loader = self.loader.clone();
        resize_vec(self.max_loader_len as usize, &mut loader, 0);
        disk[0]
            .data
            .splice(0..(self.max_loader_len - 1) as usize, loader);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Fn {
    name: String,
    blocks: Vec<Vec<Bytecode>>,
    entrypoint: usize,
    id: usize,
    loc: usize,
    arg_count: Vec<usize>,
    symbol_table: SymbolTable,
    symbol_enabled: bool,
}
impl Fn {
    pub(crate) fn new(name: String, args: Vec<usize>) -> Fn {
        Fn {
            name,
            blocks: vec![],
            entrypoint: 0,
            id: 0,
            loc: 0,
            arg_count: args,
            symbol_table: SymbolTable::new(),
            symbol_enabled: true,
        }
    }
    pub fn new_with_blocks(name: String, args: Vec<usize>, blocks: Vec<Vec<Bytecode>>) -> Fn {
        let mut f = Fn {
            name,
            blocks: vec![],
            entrypoint: 0,
            id: 0,
            loc: 0,
            symbol_table: SymbolTable::new(),
            symbol_enabled: true,
            arg_count: args,
        };
        for block in blocks {
            f.add_block(block, false);
        }
        f
    }

    pub(crate) fn add_block(&mut self, block: Vec<Bytecode>, entrypoint: bool) -> isize {
        self.blocks.push(block);
        if entrypoint {
            self.entrypoint = self.blocks.len() - 1;
        }
        (self.blocks.len() - 1) as isize
    }
    fn len(&self) -> usize {
        self
            .blocks
            .iter()
            .map(|b| self.get_block_len(&b))
            .sum::<usize>()
            + 5//entrypoint jump
            + match self.symbol_enabled {
                true => self.symbol_table.setup_stack().len(),
                false => 0,
            }
    }
    fn build(
        &mut self,
        pos: usize,
        fn_map: &HashMap<String, usize>,
        data_sec: usize,
        consts: &ConstantTable,
        debug: bool,
    ) -> Vec<i16> {
        let mut block_map: HashMap<usize, usize> = HashMap::new();
        let mut bytecode = Vec::new();
        let symbol_tbl_len = match self.symbol_enabled {
            true => self.symbol_table.setup_stack().len(),
            false => 0,
        };
        if self.symbol_enabled {
            bytecode.extend(self.symbol_table.setup_stack());
        }
        self.blocks
            .iter()
            .enumerate()
            .fold(pos + 5 + symbol_tbl_len, |acc, (i, b)| {
                block_map.insert(i, acc);
                acc + self.get_block_len(b)
            });
        bytecode.push(19);
        bytecode.extend_from_slice(&pack_i32(block_map[&(self.entrypoint)] as i32));
        //dbg!(self.symbol_table.len(), &self.name);
        if debug {
            println!("-------Function {}-------", self.name);
            println!(
                "-------Symbol Table-------\n{:?}",
                self.symbol_table.symbols
            );
            println!("-------Constants-------\n{:?}", consts.data_sec);
            println!("-------Bytecode-------");
            println!("_header:");
            display_bytecode(&bytecode, 0);
        }
        for (i, block) in self.blocks.iter_mut().enumerate() {
            let block_code = flatten_vec(
                block
                    .iter()
                    .map(|inst| match inst {
                        Command(c) => match c {
                            _ => vec![pack_command(*c)],
                        },
                        SymbolSectionLen() => pack_i32(self.symbol_table.len() as i32),
                        Register(r) => pack_register(*r),
                        Float(f) => pack_float(*f),
                        Int(i) => vec![*i],
                        FunctionRef(f) => pack_i32(fn_map[f] as i32),
                        ConstantLoc(c) => {
                            pack_i32((data_sec + consts.get_constant_offset(*c)) as i32)
                        }
                        BlockLoc(b) => {
                            if *b != -1 {
                                pack_i32(block_map[&(*b as usize)] as i32)
                            } else {
                                pack_i32(block_map[&(i as usize)] as i32)
                            }
                        }
                        Int32(i) => pack_i32(*i),
                        Bytecode::Symbol(name, offset) => {
                            let loc = self.symbol_table.get_symbol(name) as i32 + *offset;
                            if self.name != "main" {
                                pack_i32(loc + 2 + 2 + 5 + 4) //arp & return addr & registers r1-r5 then f1-f2
                            } else {
                                pack_i32(loc)
                            }
                        }
                        Argument(arg) => pack_i32(
                            -(self
                                .arg_count
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| i >= arg)
                                .map(|(_, x)| *x as i32)
                                .sum::<i32>()),
                        ),
                        ArgCount() => pack_i32(self.arg_count.len() as i32),
                        HeapStart() => pack_i32((data_sec + consts.len()) as i32),
                    })
                    .collect::<Vec<Vec<i16>>>(),
            );
            if debug {
                println!("Block {}:", i);
                display_bytecode(&(block_code.iter().map(|x| *x).collect()), bytecode.len());
            }
            bytecode.extend(block_code.iter().map(|x| *x));
        }

        bytecode
    }
    fn get_block_len(&self, block: &Vec<Bytecode>) -> usize {
        block
            .iter()
            .map(|inst| match inst {
                Command(_c) => 1,
                Register(_r) => 3,
                Float(_f) => 4,
                Int(_i) => 1,
                FunctionRef(_f) => 4,
                ConstantLoc(_c) => 4,
                BlockLoc(_b) => 4,
                Int32(_i) => 4,
                Bytecode::Symbol(_s, _o) => 4,
                SymbolSectionLen() => 4,
                Argument(_a) => 4,
                ArgCount() => 4,
                HeapStart() => 4,
            })
            .collect::<Vec<usize>>()
            .iter()
            .sum()
    }
    pub fn add_symbol(&mut self, name: &str, size: usize) {
        self.symbol_table
            .add_symbol(Symbol::new(name.to_string(), size));
    }
}
fn display_bytecode(bytecode: &Vec<i16>, offset: usize) {
    for (i, chunk) in bytecode.chunks(32).map(|slice| slice.to_vec()).enumerate() {
        println!("{:07}: {:?}", (i * 32) + offset, chunk);
    }
}
