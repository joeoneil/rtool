use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    fs::File,
    io::{Read, Write},
    mem::transmute,
    os::unix::fs::OpenOptionsExt,
};

use self::mem::{Memory, Page, PageID};
use crate::common::{Error, Instruction, ObjectModule};

pub use exec::Exec;

mod exec;
mod mem;

const TEXT_START: u32 = 0x00400000;
const DATA_START: u32 = 0x10000000;
const STACK_START: u32 = 0x7fffeffc;
const PAGE_BITS: u32 = 12;
const PAGE_SIZE: u32 = 1 << PAGE_BITS;
const PAGE_MASK: u32 = PAGE_SIZE - 1;
const STACK_SIZE: u32 = 0x00100000; // 1MB stack size

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Register {
    // Really this should be moved to common, I'll need it (or something similar) for rasm
    ZERO = 0,
    AT = 1,
    V0 = 2,
    V1 = 3,
    A0 = 4,
    A1 = 5,
    A2 = 6,
    A3 = 7,
    T0 = 8,
    T1 = 9,
    T2 = 10,
    T3 = 11,
    T4 = 12,
    T5 = 13,
    T6 = 14,
    T7 = 15,
    S0 = 16,
    S1 = 17,
    S2 = 18,
    S3 = 19,
    S4 = 20,
    S5 = 21,
    S6 = 22,
    S7 = 23,
    T8 = 24,
    T9 = 25,
    K0 = 26,
    K1 = 27,
    GP = 28,
    SP = 29,
    FP = 30,
    RA = 31,
}
