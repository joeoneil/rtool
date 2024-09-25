use std::fmt::Display;

use super::{
    register_name,
    types::{Error, Instruction},
};

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
        match self {
            Instruction::J { op, imm } => write!(
                f,
                "{} 0x{:08x}",
                match op {
                    0o02 => "j",
                    0o03 => "jal",
                    _ => unreachable!(),
                },
                imm << 2
            ),
            Instruction::I { op, rs, rt, imm } => match op {
                0o17 => write!(f, "lui ${}, 0x{:04x}", register_name(*rt), imm),
                _ => write!(
                    f,
                    "{} ${}, ${}, 0x{:04x}",
                    match op {
                        0o01 => "bcond",
                        0o04 => "beq",
                        0o05 => "bne",
                        0o06 => "blez",
                        0o07 => "bgtz",
                        0o10 => "addi",
                        0o11 => "addiu",
                        0o12 => "slti",
                        0o13 => "sltiu",
                        0o14 => "andi",
                        0o15 => "ori",
                        0o16 => "xori",
                        0o40 => "lb",
                        0o41 => "lh",
                        0o42 => "lwl",
                        0o43 => "lw",
                        0o44 => "lbu",
                        0o45 => "lhu",
                        0o46 => "lwr",
                        0o50 => "sb",
                        0o51 => "sh",
                        0o52 => "swl",
                        0o53 => "sw",
                        0o56 => "swr",
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
            } => match funct {
                0o00..=0o03 => {
                    write!(
                        f,
                        "{} ${}, ${}, {}",
                        match funct {
                            0o00 => "sll",
                            0o02 => "srl",
                            0o03 => "sra",
                            _ => unreachable!(),
                        },
                        register_name(*rd),
                        register_name(*rt),
                        shamt
                    )
                }
                0o10 => write!(f, "jr ${}", register_name(*rs)),
                0o11 => write!(f, "jalr ${}, ${}", register_name(*rs), register_name(*rd)),
                0o14 => write!(f, "syscall"),
                0o15 => write!(f, "break"),
                0o20 => write!(f, "mfhi ${}", register_name(*rd)),
                0o22 => write!(f, "mflo ${}", register_name(*rd)),
                0o21 => write!(f, "mthi ${}", register_name(*rs)),
                0o23 => write!(f, "mtlo ${}", register_name(*rs)),
                0o30..=0o33 => write!(
                    f,
                    "{} ${}, ${}",
                    match funct {
                        0o30 => "mult",
                        0o31 => "multu",
                        0o32 => "div",
                        0o33 => "divu",
                        _ => unreachable!(),
                    },
                    register_name(*rs),
                    register_name(*rt),
                ),
                _ => write!(
                    f,
                    "{} ${}, ${}, ${}",
                    match funct {
                        0o40 => "add",
                        0o41 => "addu",
                        0o42 => "sub",
                        0o43 => "subu",
                        0o44 => "and",
                        0o45 => "or",
                        0o46 => "xor",
                        0o47 => "nor",
                        0o52 => "slt",
                        0o53 => "sltu",
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
