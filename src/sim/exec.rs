use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
};

use super::{mem::Memory, SimArgs, EMPTY_ARGS, PAGE_SIZE};
use crate::{
    common::{Error, Instruction, ObjectModule},
    sim::{Register, DATA_START, STACK_START},
};

#[derive(Clone, Copy)]
struct ExecCtx {
    reg: [u32; 32],
    pc: u32,
    hi: u32,
    lo: u32,
}

pub struct Exec<'a> {
    ctx: ExecCtx,
    mem: Memory,
    heap_start: u32,
    heap_size: u32,
    heap_next_page: u32,
    exn: Option<Exception>,
    files: HashMap<u32, File>,
    next_fd: u32,
    args: &'a SimArgs,
}

#[derive(Clone)]
enum Exception {
    Syscall(u32),
    Break(u32),
    DivideByZero,
    Overflow,
    Memory(Error),
    Exit(u32),
    Timer,
}

impl<'a> Clone for Exec<'a> {
    fn clone(&self) -> Self {
        Exec {
            ctx: self.ctx,
            mem: self.mem.clone(),
            heap_start: self.heap_start,
            heap_size: self.heap_size,
            heap_next_page: self.heap_next_page,
            exn: self.exn.clone(),
            files: HashMap::new(),
            next_fd: 33,
            args: self.args,
        }
    }
}

impl<'a> Exec<'a> {
    fn exec_instruction(&mut self, i: Instruction) {
        use crate::common::instruction::opcodes::*;
        match i {
            Instruction::R {
                rs,
                rt,
                rd,
                shamt,
                funct,
            } => match funct {
                FUNCT_SLL => self.ctx.reg[rd as usize] = self.ctx.reg[rt as usize] << shamt,
                FUNCT_SRL => self.ctx.reg[rd as usize] = self.ctx.reg[rt as usize] >> shamt,
                FUNCT_SRA => {
                    self.ctx.reg[rd as usize] = (self.ctx.reg[rt as usize] as i32 >> shamt) as u32
                }
                FUNCT_SLLV => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rt as usize] << (self.ctx.reg[rs as usize] & 0x1F)
                }
                FUNCT_SRLV => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rt as usize] >> (self.ctx.reg[rs as usize] & 0x1F)
                }
                FUNCT_SRAV => {
                    self.ctx.reg[rd as usize] = (self.ctx.reg[rt as usize] as i32
                        >> (self.ctx.reg[rs as usize] & 0x1F))
                        as u32
                }
                FUNCT_JR => self.ctx.pc = self.ctx.reg[rs as usize],
                FUNCT_JALR => {
                    // assembler defaults rd to $ra if not specified
                    // but the register to link is always specified in the inst
                    self.ctx.reg[rd as usize] = self.ctx.pc;
                    self.ctx.pc = self.ctx.reg[rs as usize];
                }
                FUNCT_SYSCALL => self.raise_exn(Exception::Syscall(0)),
                FUNCT_BREAK => self.raise_exn(Exception::Syscall(0)),
                FUNCT_MFHI => self.ctx.reg[rd as usize] = self.ctx.hi,
                FUNCT_MTHI => self.ctx.hi = self.ctx.reg[rs as usize],
                FUNCT_MFLO => self.ctx.reg[rd as usize] = self.ctx.lo,
                FUNCT_MTLO => self.ctx.lo = self.ctx.reg[rs as usize],
                FUNCT_MULT => {
                    let a = self.ctx.reg[rs as usize] as i32 as i64;
                    let b = self.ctx.reg[rt as usize] as i32 as i64;
                    let res = (a * b) as u64;
                    self.ctx.hi = (res >> 32) as u32;
                    self.ctx.lo = (res & 0x0000FFFF) as u32
                }
                FUNCT_MULTU => {
                    let a = self.ctx.reg[rs as usize] as u64;
                    let b = self.ctx.reg[rt as usize] as u64;
                    let res = (a * b);
                    self.ctx.hi = (res >> 32) as u32;
                    self.ctx.lo = (res & 0x0000FFFF) as u32;
                }
                FUNCT_DIV => {
                    let a = self.ctx.reg[rs as usize] as i32;
                    let b = self.ctx.reg[rt as usize] as i32;
                    if b == 0 {
                        self.raise_exn(Exception::DivideByZero)
                    } else {
                        self.ctx.lo = (a / b) as u32;
                        self.ctx.hi = (a % b) as u32;
                    }
                }
                FUNCT_DIVU => {
                    let a = self.ctx.reg[rs as usize];
                    let b = self.ctx.reg[rt as usize];
                    if b == 0 {
                        self.raise_exn(Exception::DivideByZero)
                    } else {
                        self.ctx.lo = (a / b);
                        self.ctx.hi = (a % b);
                    }
                }
                FUNCT_ADD => {
                    match ((self.ctx.reg[rs as usize] as i32)
                        .checked_add(self.ctx.reg[rt as usize] as i32))
                    {
                        Some(v) => self.ctx.reg[rd as usize] = v as u32,
                        _ => self.raise_exn(Exception::Overflow),
                    }
                }
                FUNCT_ADDU => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rs as usize].wrapping_add(self.ctx.reg[rt as usize])
                }
                FUNCT_SUB => match (self.ctx.reg[rs as usize] as i32)
                    .checked_sub(self.ctx.reg[rt as usize] as i32)
                {
                    Some(v) => self.ctx.reg[rd as usize] = v as u32,
                    _ => self.raise_exn(Exception::Overflow),
                },
                FUNCT_SUBU => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rs as usize].wrapping_sub(self.ctx.reg[rt as usize])
                }
                FUNCT_AND => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rs as usize] & self.ctx.reg[rt as usize]
                }
                FUNCT_OR => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rs as usize] | self.ctx.reg[rt as usize]
                }
                FUNCT_XOR => {
                    self.ctx.reg[rd as usize] =
                        self.ctx.reg[rs as usize] ^ self.ctx.reg[rt as usize]
                }
                FUNCT_NOR => {
                    self.ctx.reg[rd as usize] =
                        !(self.ctx.reg[rs as usize] | self.ctx.reg[rt as usize])
                }
                FUNCT_SLT => {
                    self.ctx.reg[rd as usize] =
                        if (self.ctx.reg[rs as usize] as i32) < (self.ctx.reg[rt as usize] as i32) {
                            1
                        } else {
                            0
                        }
                }
                FUNCT_SLTU => {
                    self.ctx.reg[rd as usize] =
                        if self.ctx.reg[rs as usize] < self.ctx.reg[rt as usize] {
                            1
                        } else {
                            0
                        }
                }
                _ => unreachable!(),
            },
            Instruction::I { op, rs, rt, imm } => match op {
                OP_BCOND => match rt {
                    BCOND_BLTZ => {
                        if (self.ctx.reg[rs as usize] as i32) < 0 {
                            self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                        }
                    }
                    BCOND_BGEZ => {
                        if (self.ctx.reg[rs as usize] as i32) >= 0 {
                            self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                        }
                    }
                    BCOND_BLTZAL => {
                        if (self.ctx.reg[rs as usize] as i32) < 0 {
                            self.ctx.reg[31] = self.ctx.pc;
                            self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                        }
                    }
                    BCOND_BGEZAL => {
                        if (self.ctx.reg[rs as usize] as i32) >= 0 {
                            self.ctx.reg[31] = self.ctx.pc;
                            self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                        }
                    }
                    _ => unreachable!(),
                },
                OP_BEQ => {
                    if (self.ctx.reg[rs as usize] == self.ctx.reg[rt as usize]) {
                        self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                    }
                }
                OP_BNE => {
                    if (self.ctx.reg[rs as usize] != self.ctx.reg[rt as usize]) {
                        self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                    }
                }
                OP_BLEZ => {
                    if ((self.ctx.reg[rs as usize] as i32) <= 0) {
                        self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                    }
                }
                OP_BGTZ => {
                    if (self.ctx.reg[rs as usize] > 0) {
                        self.ctx.pc = (self.ctx.pc as i32 + ((imm as i16 as i32) << 2)) as u32
                    }
                }
                OP_ADDI => {
                    match (self.ctx.reg[rs as usize] as i32).checked_add(imm as i16 as i32) {
                        Some(v) => self.ctx.reg[rt as usize] = v as u32,
                        _ => self.raise_exn(Exception::Overflow),
                    }
                }
                OP_ADDIU => {
                    self.ctx.reg[rt as usize] =
                        self.ctx.reg[rs as usize].wrapping_add(imm as i16 as u32)
                }
                OP_SLTI => {
                    self.ctx.reg[rt as usize] =
                        if (self.ctx.reg[rs as usize] as i32) < (imm as i16 as i32) {
                            1
                        } else {
                            0
                        }
                }
                OP_SLTIU => {
                    self.ctx.reg[rt as usize] =
                        // need to compare unsigned (reg) against signed (imm)
                        // if (unsigned) < (negative) is always false, so
                        // false if high order bit of immediate is set.
                        if !((imm & 0x8000) > 0) && self.ctx.reg[rs as usize] < (imm as u32) {
                            1
                        } else {
                            0
                        }
                }
                OP_ANDI => self.ctx.reg[rt as usize] = self.ctx.reg[rs as usize] & (imm as u32),
                OP_ORI => self.ctx.reg[rt as usize] = self.ctx.reg[rs as usize] | (imm as u32),
                OP_XORI => self.ctx.reg[rt as usize] = self.ctx.reg[rs as usize] ^ (imm as u32),
                OP_LUI => self.ctx.reg[rt as usize] = (imm as u32) << 16,
                OP_LB => {
                    let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                    match self.mem.read_byte(a) {
                        Ok(v) => self.ctx.reg[rt as usize] = v as i8 as u32,
                        Err(e) => self.raise_exn(Exception::Memory(e)),
                    }
                }
                OP_LH => {
                    let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                    match self.mem.read_half(a) {
                        Ok(v) => self.ctx.reg[rt as usize] = v as i16 as u32,
                        Err(e) => self.raise_exn(Exception::Memory(e)),
                    }
                }
                OP_LWL => {
                    todo!()
                }
                OP_LW => {
                    let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                    match self.mem.read_word(a) {
                        Ok(v) => self.ctx.reg[rt as usize] = v,
                        Err(e) => self.raise_exn(Exception::Memory(e)),
                    }
                }
                OP_LBU => {
                    let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                    match self.mem.read_byte(a) {
                        Ok(v) => self.ctx.reg[rt as usize] = v as u32,
                        Err(e) => self.raise_exn(Exception::Memory(e)),
                    }
                }
                OP_LHU => {
                    let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                    match self.mem.read_half(a) {
                        Ok(v) => self.ctx.reg[rt as usize] = v as u32,
                        Err(e) => self.raise_exn(Exception::Memory(e)),
                    }
                }
                OP_LWR => {
                    todo!()
                }
                OP_SB => {
                    let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                    match self.mem.write_byte(a, self.ctx.reg[rt as usize] as u8) {
                        Ok(()) => {}
                        Err(e) => self.raise_exn(Exception::Memory(e)),
                    }
                }
                OP_SH => {
                    let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                    match self.mem.write_half(a, self.ctx.reg[rt as usize] as u16) {
                        Ok(()) => {}
                        Err(e) => self.raise_exn(Exception::Memory(e)),
                    }
                }
                OP_SWL => {
                    todo!()
                }
                OP_SW => {
                    let a = (self.ctx.reg[rs as usize] as i32 + (imm as i16 as i32)) as u32;
                    match self.mem.write_word(a, self.ctx.reg[rt as usize]) {
                        Ok(()) => {}
                        Err(e) => self.raise_exn(Exception::Memory(e)),
                    }
                }
                OP_SWR => {
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
            },
            Instruction::J { op, imm } => match op {
                OP_J => self.ctx.pc = ((self.ctx.pc & 0xF0000000) | (imm << 2)) - 4,
                OP_JAL => {
                    self.ctx.reg[31] = self.ctx.pc;
                    self.ctx.pc = (self.ctx.pc & 0xF0000000 | (imm << 2)) - 4
                }
                _ => unreachable!(),
            },
        }
    }

    fn raise_exn(&mut self, exn: Exception) {
        if !self.args.no_kern_clobber {
            const MASK: u32 = 0b10000000000000000000000001100010;
            if self.ctx.reg[Register::K0 as usize] & 1 > 0 {
                self.ctx.reg[Register::K0 as usize] ^= MASK;
            }
            self.ctx.reg[Register::K0 as usize] >>= 1;
            if self.ctx.reg[Register::K1 as usize] & 1 > 0 {
                self.ctx.reg[Register::K1 as usize] ^= MASK;
            }
            self.ctx.reg[Register::K1 as usize] >>= 1;
        }

        match exn {
            Exception::Syscall(v) | Exception::Break(v) => self.syscall(v),
            // stores exn to be checked before executing next instruction
            e => self.exn = Some(e),
        }
    }

    fn syscall(&mut self, _imm: u32) {
        use crate::common::instruction::opcodes::*;

        match self.ctx.reg[Register::V0 as usize] {
            // print_int
            SYSCALL_PRINT_INT => {
                print!("{}", self.ctx.reg[Register::A0 as usize]);
                std::io::stdout().flush().unwrap();
            }
            // print_string(buf)
            SYSCALL_PRINT_STRING => {
                let mut a = self.ctx.reg[Register::A0 as usize];
                match self.read_string(a) {
                    Ok(s) => {
                        print!("{}", s);
                        std::io::stdout().flush();
                    }
                    Err(e) => self.exn = Some(Exception::Memory(e)),
                }
            }
            // read_int
            SYSCALL_READ_INT => {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).unwrap();
                line = line.chars().take_while(|c| c.is_ascii_digit()).collect();
                match line.parse::<i32>() {
                    Ok(i) => {
                        self.ctx.reg[Register::V0 as usize] = i as u32;
                        self.ctx.reg[Register::V1 as usize] = 0;
                    }
                    Err(_) => {
                        self.ctx.reg[Register::V1 as usize] = 1;
                    }
                }
            }
            // read_string(buf, len)
            SYSCALL_READ_STRING => {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).unwrap();
                let bytes = line.as_bytes();
                let mut buf_addr = self.ctx.reg[Register::A0 as usize];
                self.ctx.reg[Register::V0 as usize] = buf_addr;
                let len = self.ctx.reg[Register::A1 as usize];
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
                    self.ctx.reg[Register::V0 as usize] = 0;
                }
            }
            // sbrk(amt)
            SYSCALL_SBRK => {
                self.ctx.reg[Register::V0 as usize] = self.heap_start;
                if self.ctx.reg[Register::A0 as usize] != 0 {
                    let new_pages = (self.ctx.reg[4] + PAGE_SIZE - 1) / PAGE_SIZE;
                    for _ in 0..new_pages {
                        self.mem.alloc_page(self.heap_next_page, true, false);
                        self.heap_next_page += PAGE_SIZE;
                    }
                    self.heap_size += new_pages * PAGE_SIZE;
                }
                self.ctx.reg[Register::V1 as usize] = self.heap_size;
            }
            // exit()
            SYSCALL_EXIT => {
                self.exn = Some(Exception::Exit(0));
            }
            // print_char(char)
            SYSCALL_PRINT_CHAR => {
                print!("{}", char::from(self.ctx.reg[Register::A0 as usize] as u8));
                std::io::stdout().flush().unwrap();
            }
            // read_char()
            SYSCALL_READ_CHAR => {
                let mut byte = [0u8];
                std::io::stdin().read_exact(&mut byte);
                self.ctx.reg[Register::A0 as usize] = byte[0] as u32;
            }
            // open(name, flags, mode)
            SYSCALL_OPEN => {
                let mut opts = std::fs::OpenOptions::new();
                let name = match self.read_string(self.ctx.reg[Register::A0 as usize]) {
                    Ok(s) => s,
                    Err(e) => {
                        self.exn = Some(Exception::Memory(e));
                        return;
                    }
                };
                let flags = self.ctx.reg[Register::A1 as usize];
                let mode = self.ctx.reg[Register::A2 as usize];
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
                        self.ctx.reg[Register::V0 as usize] = self.next_fd;
                        self.next_fd += 1;
                    }
                    Err(e) => {
                        self.ctx.reg[Register::V0 as usize] = -1i32 as u32;
                    }
                }
            }
            // read(fd, buf, len)
            SYSCALL_READ => {
                if let Some(f) = self.files.get_mut(&self.ctx.reg[Register::A0 as usize]) {
                    let mut buf: Vec<u8> =
                        Vec::with_capacity(self.ctx.reg[Register::A2 as usize] as usize);
                    let read = match f.read(buf.as_mut_slice()) {
                        Ok(amt) => amt,
                        Err(_) => {
                            self.ctx.reg[Register::V0 as usize] = -1i32 as u32;
                            return;
                        }
                    };
                    self.ctx.reg[Register::V0 as usize] = read as u32;
                    for (off, b) in buf.iter().enumerate().take(read) {
                        match self
                            .mem
                            .write_byte(self.ctx.reg[Register::A1 as usize] + off as u32, *b)
                        {
                            Ok(_) => {}
                            Err(e) => {
                                self.exn = Some(Exception::Memory(e));
                                break;
                            }
                        }
                    }
                } else {
                    self.ctx.reg[Register::V0 as usize] = -1i32 as u32;
                }
            }
            // write(fd, buf, len)
            SYSCALL_WRITE => {
                if let Some(f) = self.files.get_mut(&self.ctx.reg[Register::A0 as usize]) {
                    let mut buf: Vec<u8> =
                        Vec::with_capacity(self.ctx.reg[Register::A2 as usize] as usize);
                    for off in 0..self.ctx.reg[6] as usize {
                        buf.push(
                            match self
                                .mem
                                .read_byte(self.ctx.reg[Register::A1 as usize] + off as u32)
                            {
                                Ok(b) => b,
                                Err(e) => {
                                    self.exn = Some(Exception::Memory(e));
                                    break;
                                }
                            },
                        )
                    }
                    match f.write(buf.as_slice()) {
                        Ok(amt) => self.ctx.reg[Register::V0 as usize] = amt as u32,
                        Err(e) => self.ctx.reg[Register::V0 as usize] = -1i32 as u32,
                    }
                }
            }
            // close(fd)
            SYSCALL_CLOSE => {
                if self
                    .files
                    .contains_key(&self.ctx.reg[Register::A0 as usize])
                {
                    // causes the file to be dropped, which closes the fd
                    self.files.remove(&self.ctx.reg[Register::A0 as usize]);
                }
            }
            // exit2(code)
            SYSCALL_EXIT2 => self.exn = Some(Exception::Exit(self.ctx.reg[Register::A0 as usize])),
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

    pub(super) fn new_empty() -> Self {
        Self {
            ctx: ExecCtx {
                reg: [0; 32],
                pc: 0,
                hi: 0,
                lo: 0,
            },
            mem: Memory::new(),
            exn: None,
            files: HashMap::new(),
            next_fd: 3,
            heap_next_page: 0,
            heap_size: 0,
            heap_start: 0,
            args: &EMPTY_ARGS,
        }
    }

    pub fn new(module: ObjectModule, args: &'a SimArgs) -> Option<Self> {
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

        if !args.no_kern_clobber {
            ctx.reg[Register::K0 as usize] = 0x00000000;
            ctx.reg[Register::K1 as usize] = 0xFFFFFFFF;
        }

        let mem = Memory::new_from_object(module, args);

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
            args,
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
        if self.args.trace {
            eprintln!("pc @ 0x{:08x}: 0x{:08x} -> {}", self.ctx.pc, i, inst);
        }
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

        self.ctx.reg[Register::ZERO as usize] = 0;
        self.ctx.pc += 4;
        Ok(())
    }
}
