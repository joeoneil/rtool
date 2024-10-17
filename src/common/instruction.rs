use std::fmt::Display;

use super::{
    register_name,
    types::{Error, Instruction},
};

pub mod opcodes {
    pub const FUNCT_SLL: u8 = 0o00;
    pub const FUNCT_SRL: u8 = 0o02;
    pub const FUNCT_SRA: u8 = 0o03;
    pub const FUNCT_SLLV: u8 = 0o04;
    pub const FUNCT_SRLV: u8 = 0o06;
    pub const FUNCT_SRAV: u8 = 0o07;
    pub const FUNCT_JR: u8 = 0o10;
    pub const FUNCT_JALR: u8 = 0o11;
    pub const FUNCT_SYSCALL: u8 = 0o14;
    pub const FUNCT_BREAK: u8 = 0o15;
    pub const FUNCT_MFHI: u8 = 0o20;
    pub const FUNCT_MTHI: u8 = 0o21;
    pub const FUNCT_MFLO: u8 = 0o22;
    pub const FUNCT_MTLO: u8 = 0o23;
    pub const FUNCT_MULT: u8 = 0o30;
    pub const FUNCT_MULTU: u8 = 0o31;
    pub const FUNCT_DIV: u8 = 0o32;
    pub const FUNCT_DIVU: u8 = 0o33;
    pub const FUNCT_ADD: u8 = 0o40;
    pub const FUNCT_ADDU: u8 = 0o41;
    pub const FUNCT_SUB: u8 = 0o42;
    pub const FUNCT_SUBU: u8 = 0o43;
    pub const FUNCT_AND: u8 = 0o44;
    pub const FUNCT_OR: u8 = 0o45;
    pub const FUNCT_XOR: u8 = 0o46;
    pub const FUNCT_NOR: u8 = 0o47;
    pub const FUNCT_SLT: u8 = 0o52;
    pub const FUNCT_SLTU: u8 = 0o53;

    pub const OP_FUNCT: u8 = 0o00;
    pub const OP_BCOND: u8 = 0o01;
    pub const OP_J: u8 = 0o02;
    pub const OP_JAL: u8 = 0o03;
    pub const OP_BEQ: u8 = 0o04;
    pub const OP_BNE: u8 = 0o05;
    pub const OP_BLEZ: u8 = 0o06;
    pub const OP_BGTZ: u8 = 0o07;
    pub const OP_ADDI: u8 = 0o10;
    pub const OP_ADDIU: u8 = 0o11;
    pub const OP_SLTI: u8 = 0o12;
    pub const OP_SLTIU: u8 = 0o13;
    pub const OP_ANDI: u8 = 0o14;
    pub const OP_ORI: u8 = 0o15;
    pub const OP_XORI: u8 = 0o16;
    pub const OP_LUI: u8 = 0o17;
    pub const OP_LB: u8 = 0o40;
    pub const OP_LH: u8 = 0o41;
    pub const OP_LWL: u8 = 0o42;
    pub const OP_LW: u8 = 0o43;
    pub const OP_LBU: u8 = 0o44;
    pub const OP_LHU: u8 = 0o45;
    pub const OP_LWR: u8 = 0o46;
    pub const OP_SB: u8 = 0o50;
    pub const OP_SH: u8 = 0o51;
    pub const OP_SWL: u8 = 0o52;
    pub const OP_SW: u8 = 0o53;
    pub const OP_SWR: u8 = 0o56;

    pub const BCOND_BLTZ: u8 = 0o00;
    pub const BCOND_BGEZ: u8 = 0o01;
    pub const BCOND_BLTZAL: u8 = 0o20;
    pub const BCOND_BGEZAL: u8 = 0o21;

    pub const SYSCALL_PRINT_INT: u32 = 1;
    pub const SYSCALL_PRINT_STRING: u32 = 4;
    pub const SYSCALL_READ_INT: u32 = 5;
    pub const SYSCALL_READ_STRING: u32 = 8;
    pub const SYSCALL_SBRK: u32 = 9;
    pub const SYSCALL_EXIT: u32 = 10;
    pub const SYSCALL_PRINT_CHAR: u32 = 11;
    pub const SYSCALL_READ_CHAR: u32 = 12;
    pub const SYSCALL_OPEN: u32 = 13;
    pub const SYSCALL_READ: u32 = 14;
    pub const SYSCALL_WRITE: u32 = 15;
    pub const SYSCALL_CLOSE: u32 = 16;
    pub const SYSCALL_EXIT2: u32 = 17;
    pub const SYSCALL_SNAP: u32 = 18;
    pub const SYSCALL_RSNAP: u32 = 19;
}

/// Extracts a bitfield from a 32-bit number, idx 0 is the highest order bit.
/// idx 31 is the lowest order bit.
const fn extract_bits(val: u32, idx: u8, len: u8) -> u32 {
    (val << idx) >> (32 - len)
}

impl TryFrom<u32> for Instruction {
    type Error = super::types::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let opcode = extract_bits(value, 0, 6);
        let rs = extract_bits(value, 6, 5);
        let rt = extract_bits(value, 11, 5);
        let rd = extract_bits(value, 16, 5);
        let shamt = extract_bits(value, 21, 5);
        let funct = extract_bits(value, 26, 6);

        let imm_i = extract_bits(value, 16, 16);
        let imm_j = extract_bits(value, 6, 26);

        match opcode {
            /* R type ALU instruction */
            0 => {
                match funct {
                    // shift
                    0o00 | 0o02..=0o04 | 0o06..=0o07 => {}
                    // jump
                    0o10 | 0o11 => {}
                    // syscall / brk
                    0o14 | 0o15 => {}
                    // hi / lo reg operands
                    0o20..=0o23 => {}
                    // mult / div
                    0o30..=0o33 => {}
                    // arith
                    0o40..=0o47 => {}
                    // set cond
                    0o52..=0o53 => {}
                    _ => {
                        return Err(Error::InstructionParseError(format!(
                            "Illegal funct {:06b}",
                            funct
                        )))
                    }
                }
                Ok(Instruction::R {
                    rs: rs as u8,
                    rt: rt as u8,
                    rd: rd as u8,
                    shamt: shamt as u8,
                    funct: funct as u8,
                })
            }
            /* J type instruction */
            0o02 | 0o03 => Ok(Instruction::J {
                op: opcode as u8,
                imm: imm_j,
            }),
            /* I type instruction */
            0o04..=0o17 | 0o40..=0o46 | 0o50..=0o53 | 0o56 => Ok(Instruction::I {
                op: opcode as u8,
                rs: rs as u8,
                rt: rt as u8,
                imm: imm_i as u16,
            }),
            _ => Err(Error::InstructionParseError(format!(
                "Illegal opcode {}",
                opcode
            ))),
        }
    }
}

impl From<Instruction> for u32 {
    fn from(value: Instruction) -> Self {
        match value {
            Instruction::R {
                rs,
                rt,
                rd,
                shamt,
                funct,
            } => {
                (rs as u32) << 21
                    | (rt as u32) << 16
                    | (rd as u32) << 11
                    | (shamt as u32) << 6
                    | (funct as u32)
            }
            Instruction::I { op, rs, rt, imm } => {
                (op as u32) << 26 | (rs as u32) << 21 | (rt as u32) << 16 | (imm as u32)
            }
            Instruction::J { op, imm } => (op as u32) << 26 | imm,
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use opcodes::*;
        match self {
            Instruction::J { op, imm } => write!(
                f,
                "{} 0x{:08x}",
                match *op {
                    OP_J => "j",
                    OP_JAL => "jal",
                    _ => unreachable!(),
                },
                imm << 2
            ),
            Instruction::I { op, rs, rt, imm } => match *op {
                OP_LUI => write!(f, "lui ${}, 0x{:04x}", register_name(*rt), imm),
                _ => write!(
                    f,
                    "{} ${}, ${}, 0x{:04x}",
                    match *op {
                        // TODO: reformat this so that bcond is represented
                        // correctly. the `rt` field indicates which condition
                        // it being used, and only 2 operands are relevant
                        // visible
                        OP_BCOND => "bcond",
                        OP_BEQ => "beq",
                        OP_BNE => "bne",
                        OP_BLEZ => "blez",
                        OP_BGTZ => "bgtz",
                        OP_ADDI => "addi",
                        OP_ADDIU => "addiu",
                        OP_SLTI => "slti",
                        OP_SLTIU => "sltiu",
                        OP_ANDI => "andi",
                        OP_ORI => "ori",
                        OP_XORI => "xori",
                        OP_LB => "lb",
                        OP_LH => "lh",
                        OP_LWL => "lwl",
                        OP_LW => "lw",
                        OP_LBU => "lbu",
                        OP_LHU => "lhu",
                        OP_LWR => "lwr",
                        OP_SB => "sb",
                        OP_SH => "sh",
                        OP_SWL => "swl",
                        OP_SW => "sw",
                        OP_SWR => "swr",
                        _ => unreachable!(),
                    },
                    register_name(*rt),
                    register_name(*rs),
                    imm
                ),
            },
            Instruction::R {
                rs,
                rt,
                rd,
                shamt,
                funct,
            } => match *funct {
                0o00..=0o03 => {
                    write!(
                        f,
                        "{} ${}, ${}, {}",
                        match *funct {
                            FUNCT_SLL => "sll",
                            FUNCT_SRL => "srl",
                            FUNCT_SRA => "sra",
                            _ => unreachable!(),
                        },
                        register_name(*rd),
                        register_name(*rt),
                        shamt
                    )
                }
                FUNCT_JR => write!(f, "jr ${}", register_name(*rs)),
                FUNCT_JALR => write!(f, "jalr ${}, ${}", register_name(*rs), register_name(*rd)),
                FUNCT_SYSCALL => write!(f, "syscall"),
                FUNCT_BREAK => write!(f, "break"),
                FUNCT_MFHI => write!(f, "mfhi ${}", register_name(*rd)),
                FUNCT_MFLO => write!(f, "mflo ${}", register_name(*rd)),
                FUNCT_MTHI => write!(f, "mthi ${}", register_name(*rs)),
                FUNCT_MTLO => write!(f, "mtlo ${}", register_name(*rs)),
                0o30..=0o33 => write!(
                    f,
                    "{} ${}, ${}",
                    match *funct {
                        FUNCT_MULT => "mult",
                        FUNCT_MULTU => "multu",
                        FUNCT_DIV => "div",
                        FUNCT_DIVU => "divu",
                        _ => unreachable!(),
                    },
                    register_name(*rs),
                    register_name(*rt),
                ),
                _ => write!(
                    f,
                    "{} ${}, ${}, ${}",
                    match *funct {
                        FUNCT_ADD => "add",
                        FUNCT_ADDU => "addu",
                        FUNCT_SUB => "sub",
                        FUNCT_SUBU => "subu",
                        FUNCT_AND => "and",
                        FUNCT_OR => "or",
                        FUNCT_XOR => "xor",
                        FUNCT_NOR => "nor",
                        FUNCT_SLT => "slt",
                        FUNCT_SLTU => "sltu",
                        _ => unreachable!(),
                    },
                    register_name(*rd),
                    register_name(*rs),
                    register_name(*rt)
                ),
                _ => todo!(),
            },
        }
    }
}
