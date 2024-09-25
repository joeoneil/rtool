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

#[derive(Clone, PartialEq, Eq)]
pub struct SymEntry {
    pub flags: u32,
    pub val: u32,
    pub str_off: u32,
    pub ofid: u16,
}

#[derive(Clone, PartialEq, Eq)]
pub struct RelEntry {
    pub addr: u32,
    pub sect: u8,
    pub rel_info: u32,
}

#[derive(Clone, PartialEq, Eq)]
pub struct RefEntry {
    pub addr: u32,
    pub str_off: u32,
    pub ref_info: u32,
}
