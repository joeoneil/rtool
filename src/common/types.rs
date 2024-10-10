use std::{ffi::CString, fmt::Display};

/// Unified error type across all rtool subcommands
#[derive(Clone, Debug)]
pub enum Error {
    InstructionParseError(String),
    MemoryAccessError(String),
    UnhandledException(String),
}

/// Intermediate instruction representation allowing easy conversion to and
/// from binary data, as well as easier decoding to real instructions when
/// simulating.
#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    R {
        rs: u8,
        rt: u8,
        rd: u8,
        shamt: u8,
        funct: u8,
    },
    I {
        op: u8,
        rs: u8,
        rt: u8,
        imm: u16,
    },
    J {
        op: u8,
        imm: u32,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ObjectHeader {
    /// magic number. Should be 0xface
    pub(crate) magic: u16,
    /// object module format version
    pub(crate) version: u16,
    /// module content flags
    pub(crate) flags: u32,
    /// module entry point
    pub(crate) entry: u32,
    /// sections sizes.
    pub(crate) data: [u32; 10],
}

#[derive(Clone, PartialEq, Eq)]
pub struct ObjectModule {
    pub(crate) head: ObjectHeader,
    pub(crate) text: Vec<u8>,
    pub(crate) rdata: Vec<u8>,
    pub(crate) data: Vec<u8>,
    pub(crate) sdata: Vec<u8>,
    pub(crate) rel_info: Vec<RelEntry>,
    pub(crate) ext_ref: Vec<RefEntry>,
    pub(crate) symtab: Vec<SymEntry>,
    pub(crate) strtab: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Location {
    TEXT = 0,
    RDATA = 1,
    DATA = 2,
    SDATA = 3,
    SBSS = 4,
    BSS = 5,
    REL = 6,
    REF = 7,
    SYM = 8,
    STR = 9,
    HEAP = 10,
    STACK = 11,
    ABS = 12,
    EXT = 13,
    UNK = 14,
    NONE = 15,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SymEntry {
    pub flags: u32,
    pub val: u32,
    pub str_off: u32,
    pub ofid: u16,
}

pub const SYM_FORW: u32 = 0x0000_0010;
pub const SYM_RELOC: u32 = 0x0000_0020;
pub const SYM_EQ: u32 = 0x0000_0040;
pub const SYM_LBL: u32 = 0x0000_0080;
pub const SYM_REG: u32 = 0x0000_0100;
pub const SYM_PRE: u32 = 0x0000_0200;
pub const SYM_UNDEF: u32 = 0x0000_0400;
pub const SYM_XTV: u32 = 0x0000_0800;
pub const SYM_MUL: u32 = 0x0000_1000;
pub const SYM_RPT: u32 = 0x0000_2000;
pub const SYM_GLB: u32 = 0x0000_4000;
pub const SYM_SML: u32 = 0x0000_8000;
pub const SYM_ADJ: u32 = 0x0001_0000;
pub const SYM_DISC: u32 = 0x0002_0000;
pub const SYM_LIT: u32 = 0x0004_0000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RelEntry {
    pub addr: u32,
    pub sect: Location,
    pub rel_info: RefType,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RefEntry {
    pub addr: u32,
    pub str_off: u32,
    pub ref_info: RefInfo,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RefInfo {
    pub ix: u16,
    pub unknown: RefUnknown,
    pub typ: RefType,
    pub sect: Location,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RefUnknown {
    PLUS = 0,
    EQ = 1,
    MINUS = 2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RefType {
    IMM = 1,
    IMM2 = 2,
    WORD = 3,
    JUMP = 4,
    IMM3 = 5,
}
