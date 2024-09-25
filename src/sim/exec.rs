use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
};

use super::{mem::Memory, PAGE_SIZE};
use crate::{
    common::{Error, Instruction, ObjectModule},
    sim::{Register, DATA_START, STACK_START},
};

struct ExecCtx {
    reg: [u32; 32],
    pc: u32,
    hi: u32,
    lo: u32,
}
pub struct Exec {
    ctx: ExecCtx,
    mem: Memory,
    heap_start: u32,
    heap_size: u32,
    heap_next_page: u32,
    exn: Option<Exception>,
    files: HashMap<u32, File>,
    next_fd: u32,
}

enum Exception {
    Syscall(u32),
    Break(u32),
    DivideByZero,
    Overflow,
    Memory(Error),
    Exit(u32),
    Timer,
}

impl Exec {
    fn exec_instruction(&mut self, i: Instruction) {
        match i {
            Instruction::R {
                rs,
                rt,
                rd,
                shamt,
                funct,
            } => match funct {
                // sll
                0o00 => self.ctx.reg[rd as usize] = self.ctx.reg[rt as usize] << shamt,
                // srl
                0o02 => self.ctx.reg[rd as usize] = self.ctx.reg[rt as usize] >> shamt,
                // sra
                0o03 => {
                    self.ctx.reg[rd as usize] = (self.ctx.reg[rt as usize] as i32 >> shamt) as u32
                }
                // sllv
                0o04 => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rt as usize] << (self.ctx.reg[rs as usize] & 0x1F)
                }
                // srlv
                0o06 => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rt as usize] >> (self.ctx.reg[rs as usize] & 0x1F)
                }
                // srav
                0o07 => {
                    self.ctx.reg[rd as usize] = (self.ctx.reg[rt as usize] as i32
                        >> (self.ctx.reg[rs as usize] & 0x1F))
                        as u32
                }
                // jr
                0o10 => self.ctx.pc = self.ctx.reg[rs as usize],
                // jalr
                0o11 => {
                    // assembler defaults rd to $ra if not specified
                    self.ctx.reg[rd as usize] = self.ctx.pc;
                    self.ctx.pc = self.ctx.reg[rs as usize];
                }
                // syscall
                0o14 => self.raise_exn(Exception::Syscall(0)),
                // break
                0o15 => self.raise_exn(Exception::Syscall(0)),
                // mfhi
                0o20 => self.ctx.reg[rd as usize] = self.ctx.hi,
                // mthi
                0o21 => self.ctx.hi = self.ctx.reg[rs as usize],
                // mflo
                0o22 => self.ctx.reg[rd as usize] = self.ctx.lo,
                // mtlo
                0o23 => self.ctx.lo = self.ctx.reg[rs as usize],
                // mult
                0o30 => {
                    let a = self.ctx.reg[rs as usize] as i32 as i64;
                    let b = self.ctx.reg[rt as usize] as i32 as i64;
                    let res = (a * b) as u64;
                    self.ctx.hi = (res >> 32) as u32;
                    self.ctx.lo = (res & 0x0000FFFF) as u32
                }
                // multu
                0o31 => {
                    let a = self.ctx.reg[rs as usize] as u64;
                    let b = self.ctx.reg[rt as usize] as u64;
                    let res = (a * b);
                    self.ctx.hi = (res >> 32) as u32;
                    self.ctx.lo = (res & 0x0000FFFF) as u32;
                }
                // div
                0o32 => {
                    let a = self.ctx.reg[rs as usize] as i32;
                    let b = self.ctx.reg[rt as usize] as i32;
                    if b == 0 {
                        self.raise_exn(Exception::DivideByZero)
                    } else {
                        self.ctx.lo = (a / b) as u32;
                        self.ctx.hi = (a % b) as u32;
                    }
                }
                // divu
                0o33 => {
                    let a = self.ctx.reg[rs as usize];
                    let b = self.ctx.reg[rt as usize];
                    if b == 0 {
                        self.raise_exn(Exception::DivideByZero)
                    } else {
                        self.ctx.lo = (a / b);
                        self.ctx.hi = (a % b);
                    }
                }
                // add
                0o40 => {
                    match ((self.ctx.reg[rs as usize] as i32)
                        .checked_add(self.ctx.reg[rt as usize] as i32))
                    {
                        Some(v) => self.ctx.reg[rd as usize] = v as u32,
                        _ => self.raise_exn(Exception::Overflow),
                    }
                }
                // addu
                0o41 => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rs as usize].wrapping_add(self.ctx.reg[rt as usize])
                }
                // sub
                0o42 => match (self.ctx.reg[rs as usize] as i32)
                    .checked_sub(self.ctx.reg[rt as usize] as i32)
                {
                    Some(v) => self.ctx.reg[rd as usize] = v as u32,
                    _ => self.raise_exn(Exception::Overflow),
                },
                // subu
                0o43 => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rs as usize].wrapping_sub(self.ctx.reg[rt as usize])
                }
                // and
                0o44 => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rs as usize] & self.ctx.reg[rt as usize]
                }
                // or
                0o45 => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rs as usize] | self.ctx.reg[rt as usize]
                }
                // xor
                0o46 => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rs as usize] ^ self.ctx.reg[rt as usize]
                }
                // nor
                0x47 => {
                    self.ctx.reg[rd as usize] =
                        !(self.ctx.reg[rs as usize] | self.ctx.reg[rt as usize])
                }
                // slt
                0o52 => {
                    self.ctx.reg[rd as usize] =
                        if (self.ctx.reg[rs as usize] as i32) < (self.ctx.reg[rt as usize] as i32) {
                            1
                        } else {
                            0
                        }
                }
                // sltu
                0o53 => {
                    self.ctx.reg[rd as usize] =
                        if self.ctx.reg[rs as usize] < self.ctx.reg[rt as usize] {
                            1
                        } else {
                            0
                        }
                }
                _ => unreachable!(),
            },
            Instruction::I { op, rs, rt, imm } => {
                match op {
                    // BCOND special case
                    0o01 => match rt {
                        // bltz
                        0o00 => {
                            if (self.ctx.reg[rs as usize] as i32) < 0 {
                                self.ctx.pc =
                                    (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                            }
                        }
                        // bgez
                        0o01 => {
                            if (self.ctx.reg[rs as usize] as i32) >= 0 {
                                self.ctx.pc =
                                    (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                            }
                        }
                        // bltzal
                        0o20 => {
                            if (self.ctx.reg[rs as usize] as i32) < 0 {
                                self.ctx.reg[31] = self.ctx.pc;
                                self.ctx.pc =
                                    (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                            }
                        }
                        // bgezal
                        0o21 => {
                            if (self.ctx.reg[rs as usize] as i32) >= 0 {
                                self.ctx.reg[31] = self.ctx.pc;
                                self.ctx.pc =
                                    (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                            }
                        }
                        _ => unreachable!(),
                    },
                    // beq
                    0o04 => {
                        if (self.ctx.reg[rs as usize] == self.ctx.reg[rt as usize]) {
                            self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                        }
                    }
                    // bne
                    0o05 => {
                        if (self.ctx.reg[rs as usize] != self.ctx.reg[rt as usize]) {
                            self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                        }
                    }
                    // blez
                    0o06 => {
                        if ((self.ctx.reg[rs as usize] as i32) <= 0) {
                            self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                        }
                    }
                    // bgtz
                    0o07 => {
                        if (self.ctx.reg[rs as usize] > 0) {
                            self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                        }
                    }
                    // addi
                    0o10 => {
                        match (self.ctx.reg[rs as usize] as i32).checked_add(imm as i16 as i32) {
                            Some(v) => self.ctx.reg[rt as usize] = v as u32,
                            _ => self.raise_exn(Exception::Overflow),
                        }
                    }
                    // addiu
                    0o11 => {
                        self.ctx.reg[rt as usize] =
                            self.ctx.reg[rs as usize].wrapping_add(imm as u32)
                    }
                    // slti
                    0o12 => {
                        self.ctx.reg[rt as usize] =
                            if (self.ctx.reg[rs as usize] as i32) < (imm as i16 as i32) {
                                1
                            } else {
                                0
                            }
                    }
                    // sltiu
                    0o13 => {
                        self.ctx.reg[rt as usize] = if self.ctx.reg[rs as usize] < imm as u32 {
                            1
                        } else {
                            0
                        }
                    }
                    // andi
                    0o14 => self.ctx.reg[rt as usize] = self.ctx.reg[rs as usize] & (imm as u32),
                    // ori
                    0o15 => self.ctx.reg[rt as usize] = self.ctx.reg[rs as usize] | (imm as u32),
                    // xori
                    0o16 => self.ctx.reg[rt as usize] = self.ctx.reg[rs as usize] ^ (imm as u32),
                    // lui
                    0o17 => self.ctx.reg[rt as usize] = (imm as u32) << 16,
                    // lb
                    0o40 => {
                        let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                        match self.mem.read_byte(a) {
                            Ok(v) => self.ctx.reg[rt as usize] = v as i8 as u32,
                            Err(e) => self.raise_exn(Exception::Memory(e)),
                        }
                    }
                    // lh
                    0o41 => {
                        let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                        match self.mem.read_half(a) {
                            Ok(v) => self.ctx.reg[rt as usize] = v as i16 as u32,
                            Err(e) => self.raise_exn(Exception::Memory(e)),
                        }
                    }
                    // lwl
                    0o42 => {
                        todo!()
                    }
                    // lw
                    0o43 => {
                        let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                        match self.mem.read_word(a) {
                            Ok(v) => self.ctx.reg[rt as usize] = v,
                            Err(e) => self.raise_exn(Exception::Memory(e)),
                        }
                    }
                    // lbu
                    0o44 => {
                        let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                        match self.mem.read_byte(a) {
                            Ok(v) => self.ctx.reg[rt as usize] = v as u32,
                            Err(e) => self.raise_exn(Exception::Memory(e)),
                        }
                    }
                    // lhu
                    0o45 => {
                        let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                        match self.mem.read_half(a) {
                            Ok(v) => self.ctx.reg[rt as usize] = v as u32,
                            Err(e) => self.raise_exn(Exception::Memory(e)),
                        }
                    }
                    // lwr
                    0o46 => {
                        todo!()
                    }
                    // sb
                    0o50 => {
                        let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                        match self.mem.write_byte(a, self.ctx.reg[rt as usize] as u8) {
                            Ok(()) => {}
                            Err(e) => self.raise_exn(Exception::Memory(e)),
                        }
                    }
                    // sh
                    0o51 => {
                        let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                        match self.mem.write_half(a, self.ctx.reg[rt as usize] as u16) {
                            Ok(()) => {}
                            Err(e) => self.raise_exn(Exception::Memory(e)),
                        }
                    }
                    // swl
                    0o52 => {
                        todo!()
                    }
                    // sw
                    0o53 => {
                        let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                        match self.mem.write_word(a, self.ctx.reg[rt as usize]) {
                            Ok(()) => {}
                            Err(e) => self.raise_exn(Exception::Memory(e)),
                        }
                    }
                    // swr
                    0o56 => {
                        todo!()
                    }
                    0o20..=0o23 => {
                        panic!("Unimplemented coprocessor instruction")
                    }
                    0o60..=0o63 => {
                        panic!("Unimplemented load word from coprocessor instruction")
                    }
                    0o70..=0o73 => {
                        panic!("Unimplemented store word to coprocessor instruction")
                    }
                    _ => unreachable!(),
                }
            }
            Instruction::J { op, imm } => match op {
                0o02 => self.ctx.pc = ((self.ctx.pc & 0xF0000000) | (imm << 2)) - 4,
                0o03 => {
                    self.ctx.reg[31] = self.ctx.pc;
                    self.ctx.pc = (self.ctx.pc & 0xF0000000 | (imm << 2)) - 4
                }
                _ => unreachable!(),
            },
        }
    }

    fn raise_exn(&mut self, exn: Exception) {
        const MASK: u32 = 0b10000000000000000000000001100010;
        if self.ctx.reg[Register::K0 as usize] & 1 > 0 {
            self.ctx.reg[Register::K0 as usize] ^= MASK;
        }
        self.ctx.reg[Register::K0 as usize] >>= 1;
        if self.ctx.reg[Register::K1 as usize] & 1 > 0 {
            self.ctx.reg[Register::K1 as usize] ^= MASK;
        }
        self.ctx.reg[Register::K1 as usize] >>= 1;

        match exn {
            Exception::Syscall(v) | Exception::Break(v) => self.syscall(v),
            // stores exn to be checked before executing next instruction
            e => self.exn = Some(e),
        }
    }

    fn syscall(&mut self, _imm: u32) {
        match self.ctx.reg[2] {
            // print_int
            1 => {
                print!("{}", self.ctx.reg[4]);
                std::io::stdout().flush().unwrap();
            }
            // print_string(buf)
            4 => {
                let mut a = self.ctx.reg[4];
                match self.read_string(a) {
                    Ok(s) => {
                        print!("{}", s);
                        std::io::stdout().flush();
                    }
                    Err(e) => self.exn = Some(Exception::Memory(e)),
                }
            }
            // read_int
            5 => {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).unwrap();
                line = line.chars().take_while(|c| c.is_ascii_digit()).collect();
                match line.parse::<i32>() {
                    Ok(i) => {
                        self.ctx.reg[2] = i as u32;
                        self.ctx.reg[3] = 0;
                    }
                    Err(_) => {
                        self.ctx.reg[3] = 1;
                    }
                }
            }
            // read_string(buf, len)
            8 => {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).unwrap();
                let bytes = line.as_bytes();
                let mut buf_addr = self.ctx.reg[4];
                self.ctx.reg[2] = buf_addr;
                let len = self.ctx.reg[5];
                let mut read = 0;
                for b in bytes[..(len - 1) as usize].iter() {
                    match self.mem.write_byte(buf_addr, *b) {
                        Ok(_) => {
                            read += 1;
                            buf_addr += 1;
                        }
                        Err(e) => {
                            self.exn = Some(Exception::Memory(e));
                            break;
                        }
                    };
                }
                match self.mem.write_byte(buf_addr, 0) {
                    Ok(_) => {}
                    Err(e) => self.exn = Some(Exception::Memory(e)),
                }
                if read == 0 {
                    self.ctx.reg[2] = 0;
                }
            }
            // sbrk(amt)
            9 => {
                self.ctx.reg[2] = self.heap_start;
                if self.ctx.reg[4] != 0 {
                    let new_pages = (self.ctx.reg[4] + PAGE_SIZE - 1) / PAGE_SIZE;
                    for _ in 0..new_pages {
                        self.mem.alloc_page(self.heap_next_page, true, false);
                        self.heap_next_page += PAGE_SIZE;
                    }
                    self.heap_size += new_pages * PAGE_SIZE;
                }
                self.ctx.reg[3] = self.heap_size;
            }
            // exit()
            10 => {
                self.exn = Some(Exception::Exit(0));
            }
            // print_char(char)
            11 => {
                print!("{}", char::from(self.ctx.reg[4] as u8));
                std::io::stdout().flush().unwrap();
            }
            // read_char()
            12 => {
                let mut byte = [0u8];
                std::io::stdin().read_exact(&mut byte);
                self.ctx.reg[2] = byte[0] as u32;
            }
            // open(name, flags, mode)
            13 => {
                let mut opts = std::fs::OpenOptions::new();
                let name = match self.read_string(self.ctx.reg[4]) {
                    Ok(s) => s,
                    Err(e) => {
                        self.exn = Some(Exception::Memory(e));
                        return;
                    }
                };
                let flags = self.ctx.reg[5];
                let mode = self.ctx.reg[6];
                opts.mode(mode);
                if flags == 0 {
                    opts.read(true);
                }
                if flags & 0x1 != 0 {
                    opts.write(true);
                }
                if flags & 0x2 != 0 {
                    opts.read(true);
                    opts.write(true);
                }
                if flags & 0x100 != 0 {
                    opts.create(true);
                }
                if flags & 0x1000 != 0 {
                    opts.truncate(true);
                }
                opts.custom_flags(flags as i32);
                match opts.open(name) {
                    Ok(f) => {
                        self.files.insert(self.next_fd, f);
                        self.ctx.reg[2] = self.next_fd;
                        self.next_fd += 1;
                    }
                    Err(e) => {
                        self.ctx.reg[2] = -1i32 as u32;
                    }
                }
            }
            // read(fd, buf, len)
            14 => {
                if let Some(f) = self.files.get_mut(&self.ctx.reg[4]) {
                    let mut buf: Vec<u8> = Vec::with_capacity(self.ctx.reg[6] as usize);
                    let read = match f.read(buf.as_mut_slice()) {
                        Ok(amt) => amt,
                        Err(_) => {
                            self.ctx.reg[2] = -1i32 as u32;
                            return;
                        }
                    };
                    self.ctx.reg[2] = read as u32;
                    for (off, b) in buf.iter().enumerate().take(read) {
                        match self.mem.write_byte(self.ctx.reg[5] + off as u32, *b) {
                            Ok(_) => {}
                            Err(e) => {
                                self.exn = Some(Exception::Memory(e));
                                break;
                            }
                        }
                    }
                } else {
                    self.ctx.reg[2] = -1i32 as u32;
                }
            }
            // write(fd, buf, len)
            15 => {
                if let Some(f) = self.files.get_mut(&self.ctx.reg[4]) {
                    let mut buf: Vec<u8> = Vec::with_capacity(self.ctx.reg[6] as usize);
                    for off in 0..self.ctx.reg[6] as usize {
                        buf.push(match self.mem.read_byte(self.ctx.reg[5] + off as u32) {
                            Ok(b) => b,
                            Err(e) => {
                                self.exn = Some(Exception::Memory(e));
                                break;
                            }
                        })
                    }
                    match f.write(buf.as_slice()) {
                        Ok(amt) => self.ctx.reg[2] = amt as u32,
                        Err(e) => self.ctx.reg[2] = -1i32 as u32,
                    }
                }
            }
            // close(fd)
            16 => {
                if self.files.contains_key(&self.ctx.reg[4]) {
                    // causes the file to be dropped, which closes the fd
                    self.files.remove(&self.ctx.reg[4]);
                }
            }
            // exit2(code)
            17 => self.exn = Some(Exception::Exit(self.ctx.reg[4])),
            _ => unreachable!(),
        }
    }

    fn read_string(&self, mut addr: u32) -> Result<String, Error> {
        let mut bytes: Vec<u8> = vec![];
        loop {
            let b = self.mem.read_byte(addr)?;
            if b == 0 {
                break;
            }
            bytes.push(b);
            addr += 1;
        }
        Ok(String::from_utf8_lossy(bytes.as_slice()).into())
    }

    pub fn new(module: ObjectModule) -> Option<Self> {
        let mut ctx = ExecCtx {
            reg: [0; 32],
            pc: 0,
            hi: 0,
            lo: 0,
        };
        if module.head.flags & 0x3 == 0 {
            return None; // module has no entry point
        }
        ctx.pc = module.head.entry;
        // __r2k__startup__obj__ reads a bit above the stack pointer, so move it down a bit
        ctx.reg[Register::SP as usize] = STACK_START - 0x1000;
        ctx.reg[Register::FP as usize] = STACK_START;
        ctx.reg[Register::GP as usize] = DATA_START;

        ctx.reg[Register::K0 as usize] = 0x00000000;
        ctx.reg[Register::K1 as usize] = 0xFFFFFFFF;

        let mem = Memory::new_from_object(module);

        println!(
            "Creating new Execution ctx with entrypoint @ 0x{:08x}",
            ctx.pc
        );

        Some(Self {
            ctx,
            mem,
            heap_start: 0,
            heap_size: 0,
            heap_next_page: 0,
            exn: None,
            files: HashMap::new(),
            next_fd: 3,
        })
    }

    pub fn run(mut self) -> Result<(), Error> {
        loop {
            self.step()?;
        }
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), Error> {
        let i = self.mem.read_word(self.ctx.pc)?;
        let inst: Instruction = i.try_into()?;
        eprintln!("pc @ 0x{:08x}: 0x{:08x} -> {}", self.ctx.pc, i, inst);
        self.exec_instruction(inst);
        match &self.exn {
            Some(e) => {
                return Err(Error::UnhandledException(format!(
                    "Unhandled Exception: {}",
                    match e {
                        Exception::Timer => {
                            String::from("Unimplemented!?")
                        }
                        Exception::Overflow => {
                            String::from("Overflow exception")
                        }
                        Exception::Exit(code) => {
                            format!("Exit with code {}", code)
                        }
                        Exception::Syscall(operand) => {
                            format!("Syscall with operand {}", operand)
                        }
                        Exception::DivideByZero => {
                            String::from("Divide by zero")
                        }
                        Exception::Memory(Error::MemoryAccessError(e)) => {
                            format!("Memory exception: {}", e)
                        }
                        Exception::Memory(_) => unreachable!(),
                        Exception::Break(operand) => {
                            format!("Break with operand {}", operand)
                        }
                    }
                )));
            }
            None => {}
        }

        self.ctx.reg[0] = 0;
        self.ctx.pc += 4;
        Ok(())
    }
}
