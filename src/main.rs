use std::str::FromStr;
use std::time::Instant;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub type Ptr = usize;

#[derive(Debug, Clone)]
pub enum RefVal {
    Byte(u8),
    Int(i32),
    Long(i64),
    UInt(u32),
    ULong(u64),
    Str(String),
    Ptr(Ptr)
}

#[inline]
pub fn ensure_capacity(buf: &mut MutMem, req: usize) {
    if buf.available < req {
        buf.alloc(req);
    }
}

impl RefVal {
    pub fn from_str(str: &str) -> RefVal {
        let id = &str.chars().collect::<Vec<char>>()[0];
        match id {
            'L' | 'l' => RefVal::Long(i64::from_str(str.trim_start_matches(&['L', 'l'])).unwrap()),
            'I' | 'i' => RefVal::Int(i32::from_str(str.trim_start_matches(&['I', 'i'])).unwrap()),
            'u' => RefVal::UInt(u32::from_str(str.trim_start_matches('u')).unwrap()),
            'U' => RefVal::ULong(u64::from_str(str.trim_start_matches('U')).unwrap()),
            '"'  => RefVal::Str(str.trim_matches('"').to_string()),
            '*' => RefVal::Ptr(u64::from_str(str.trim_start_matches('*')).unwrap() as usize),
            'b' => RefVal::Byte(u8::from_str(str.trim_start_matches('b')).unwrap()),
            _ => panic!("Invalid val!")
        }
    }

    pub fn write(&self, buf: &mut MutMem) {
        match self {
            RefVal::Int(int) => {
                ensure_capacity(buf, 4);
                let mut tmp = Vec::<u8>::new();
                tmp.write_i32::<BigEndian>(*int).unwrap();
                buf.fill(&mut tmp);
                drop(tmp)
            }
            RefVal::Long(long) => {
                ensure_capacity(buf, 8);
                let mut tmp = Vec::<u8>::new();
                tmp.write_i64::<BigEndian>(*long).unwrap();
                buf.fill(&mut tmp);
                drop(tmp)
            }
            RefVal::UInt(int) => {
                ensure_capacity(buf, 4);
                let mut tmp = Vec::<u8>::new();
                tmp.write_u32::<BigEndian>(*int).unwrap();
                buf.fill(&mut tmp);
                drop(tmp)
            }
            RefVal::ULong(int) => {
                ensure_capacity(buf, 4);
                let mut tmp = Vec::<u8>::new();
                tmp.write_u64::<BigEndian>(*int).unwrap();
                buf.fill(&mut tmp);
                drop(tmp)
            }
            RefVal::Str(str) => {
                let mut bytes = str.to_owned().into_bytes();
                let len = bytes.len();
                ensure_capacity(buf, len + 2);
                let mut tmp = Vec::<u8>::new();
                tmp.write_u16::<BigEndian>(len as u16).unwrap();
                tmp.append(&mut bytes);
                buf.fill(&mut tmp);
                drop(tmp)
            }
            RefVal::Ptr(ptr) => {
                ensure_capacity(buf, 9);
                let mut tmp = Vec::<u8>::new();
                tmp.write_u8(0xAA).unwrap();
                tmp.write_u64::<BigEndian>(*ptr as u64).unwrap();
                buf.fill(&mut tmp);
                drop(tmp);
            }
            RefVal::Byte(byte) => {
                ensure_capacity(buf, 1);
                buf.fill(&mut vec![*byte]);
            }
        }
    }
}


#[derive(Debug, Clone)]
pub enum OpCode {
    Jmp(Ptr),
    Push(RefVal),
    Alloc(usize),
    CallV {
        v: Ptr,
        argv: Ptr
    },
    Pop,
}

impl OpCode {
    pub fn read(line: String) -> Self {
        let coll = line.split_once(" ").unwrap_or(("pop", "NIL"));
        let typ = coll.0;
        match typ {
            "jmp" => OpCode::Jmp(u64::from_str(coll.1).unwrap() as usize),
            "push" => OpCode::Push(RefVal::from_str(coll.1)),
            "alloc" => OpCode::Alloc(u64::from_str(coll.1).unwrap() as usize),
            "callv" => {
                let spl = coll.1.split_once(" ").unwrap();
                OpCode::CallV {
                    v: fn_name_to_ptr(spl.0),
                    argv: u64::from_str(spl.1).unwrap() as usize
                }
            },
            "pop" => OpCode::Pop,
            _ => panic!("Unknown expr!")
        }
    }
}

pub enum OpResult {
    Ret(RefVal),
    None
}

pub struct MutMem {
    buf: Vec<u8>,
    pos: Ptr,
    available: usize
}

impl MutMem {
    pub fn jmp(&mut self, pos: Ptr) -> OpResult {
        if pos > self.buf.len() {
            panic!("Not enough memory!")
        };

        self.pos = pos;
        OpResult::None
    }

    pub fn fill(&mut self, buf: &mut Vec<u8>) {
        buf.reverse();
        for ele in buf {
            self.buf.insert(self.pos, *ele);
        }
    }

    pub fn pop(&mut self) -> OpResult {
        let _pop = self.buf.remove(self.pos);
        OpResult::Ret(RefVal::Int(_pop as i32))
    }

    pub fn alloc(&mut self, amount: usize) -> OpResult {
        self.buf.extend(vec![0x00; amount]);
        self.available += amount;
        OpResult::None
    }

    pub fn dealloc(&mut self, pos: Ptr, amount: usize) {
        self.buf.drain(pos..pos+amount);
        self.available -= amount;
    }

    pub fn push(&mut self, val: RefVal) -> OpResult {
        val.write(self);
        OpResult::None
    }
}

macro_rules! read_mem {
    ($mem:ident, $fn:ident, $start:expr, $end:expr) => {
        (&$mem.buf[$start..$end]).$fn::<BigEndian>().unwrap()
    }
}

macro_rules! write_to_mem {
    ($mem:ident, $fn:ident, $val:ident, $pos:ident) => {
        let mut tmp: Vec<u8> = Vec::new();
        tmp.$fn::<BigEndian>($val).unwrap();
        $mem.jmp($pos);
        $mem.fill(&mut tmp);
        drop(tmp);
    }
}

pub fn add(mem: &mut MutMem, argv: Ptr) {
    let lh = read_mem!(mem, read_u64, argv, argv+8);
    let rh = read_mem!(mem, read_u64, argv+8, argv+16);
    mem.buf.drain(argv..argv+16);
    let o = lh + rh;
    write_to_mem!(mem, write_u64, o, argv);
}

pub fn mul(mem: &mut MutMem, argv: Ptr) {
    let lh = read_mem!(mem, read_u64, argv, argv+8);
    let rh = read_mem!(mem, read_u64, argv+8, argv+16);
    mem.buf.drain(argv..argv+16);
    let o = lh * rh;
    write_to_mem!(mem, write_u64, o, argv);
}

pub fn sub(mem: &mut MutMem, argv: Ptr) {
    let lh = read_mem!(mem, read_u64, argv, argv+8);
    let rh = read_mem!(mem, read_u64, argv+8, argv+16);
    mem.buf.drain(argv..argv+16);
    let o = lh - rh;
    write_to_mem!(mem, write_u64, o, argv);
}

pub fn div(mem: &mut MutMem, argv: Ptr) {
    let lh = read_mem!(mem, read_u64, argv, argv+8);
    let rh = read_mem!(mem, read_u64, argv+8, argv+16);
    mem.buf.drain(argv..argv+16);
    let o = lh / rh;
    write_to_mem!(mem, write_u64, o, argv);
}

pub fn ulong_str(mem: &mut MutMem, argv: Ptr) {
    let lh = read_mem!(mem, read_u64, argv, argv+8);
    mem.buf.drain(argv..argv+8);
    let o = lh.to_string();
    mem.jmp(argv);
    RefVal::Str(o).write(mem);
}

pub fn io_print(mem: &mut MutMem, argv: Ptr) {
    // reading str len
    let len = (&mem.buf[argv..argv+2]).read_u16::<BigEndian>().unwrap() as usize;
    let str = String::from_utf8((&mem.buf[argv+2..argv+2+len]).to_vec()).unwrap();
    print!("{}", str);
    mem.buf.drain(argv..argv+2+len);
}

pub fn callv(f: Ptr, argv: Ptr, mem: &mut MutMem) -> OpResult {
    match f {
        0x01 => io_print(mem, argv),
        0x02 => add(mem, argv),
        0x03 => mul(mem, argv),
        0x04 => sub(mem, argv),
        0x05 => div(mem, argv),
        0x06 => ulong_str(mem, argv),
        _ => panic!("Invalid virtual builtin function!")
    }
    OpResult::None
}

pub fn process(mem: &mut MutMem, data: Vec<OpCode>) {
    let mut iter = data.iter();
    while let Some(op) = iter.next() {
        process_op(mem, Clone::clone(op));
    };
}

pub fn process_op(mem: &mut MutMem, op: OpCode) -> OpResult {
    match op {
        OpCode::Jmp(pos) => mem.jmp(pos),
        OpCode::Push(val) => mem.push(val),
        OpCode::Alloc(val) => mem.alloc(val),
        OpCode::CallV { v, argv } => callv(v, argv, mem),
        OpCode::Pop => mem.pop()
    }
}

pub fn fn_name_to_ptr(name: &str) -> Ptr {
    match name {
        "printf" => 0x01,
        "add" => 0x02,
        "mul" => 0x03,
        "sub" => 0x04,
        "div" => 0x05,
        "ustr" => 0x06,
        _ => panic!("Invalid fn provided!")
    }
}

pub fn tokenize(input: &str) -> Vec<OpCode> {
    let mut buf: Vec<OpCode> = Vec::new();
    for line in input.split(";") {
        buf.push(OpCode::read(line.trim().to_string()));
    };
    buf
}

macro_rules! inline_asm {
    ($data:literal) => {
        process(&mut MutMem {
            buf: vec![],
            pos: 0x00,
            available: 0
        }, tokenize($data))
    };
}

fn main() {
    let start = Instant::now();
    inline_asm!(r#"
    push U12;
    jmp 8;
    push U22;
    callv add 0;
    callv ustr 0;
    callv printf 0;

    jmp 0;
    push U12;
    push U11000000;
    callv add 0;
    callv ustr 0;
    callv printf 0;
    "#);
    let dur = Instant::now() - start;
    println!("\nfinished in {}ms", dur.as_millis());
}
