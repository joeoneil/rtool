pub mod instruction;
pub mod module;
mod types;

use std::fmt::Display;

pub use types::*;

pub fn register_name(reg: u8) -> &'static str {
    match reg {
        0 => "zero",
        1 => "at",
        2 => "v0",
        3 => "v1",
        4 => "a0",
        5 => "a1",
        6 => "a2",
        7 => "a3",
        8 => "t0",
        9 => "t1",
        10 => "t2",
        11 => "t3",
        12 => "t4",
        13 => "t5",
        14 => "t6",
        15 => "t7",
        16 => "s0",
        17 => "s1",
        18 => "s2",
        19 => "s3",
        20 => "s4",
        21 => "s5",
        22 => "s6",
        23 => "s7",
        24 => "t8",
        25 => "t9",
        26 => "k0",
        27 => "k1",
        28 => "gp",
        29 => "sp",
        30 => "fp",
        31 => "ra",
        _ => "",
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Location::TEXT => "TEXT",
                Location::RDATA => "RDATA",
                Location::DATA => "DATA",
                Location::SDATA => "SDATA",
                Location::SBSS => "SBSS",
                Location::BSS => "BSS",
                Location::REL => "REL",
                Location::REF => "REF",
                Location::SYM => "SYM",
                Location::STR => "STR",
                Location::HEAP => "HEAP",
                Location::STACK => "STACK",
                Location::ABS => "ABS",
                Location::EXT => "EXT",
                Location::UNK => "UNK",
                Location::NONE => "NONE",
            }
        )
    }
}

pub fn has_any_flags(val: u32, flags: u32) -> bool {
    val & flags > 0
}

pub fn has_all_flags(val: u32, flags: u32) -> bool {
    val & flags == flags
}

pub fn flags_string(flags: u32) -> String {
    let mut s = String::new();
    if has_all_flags(flags, SYM_FORW) {
        s.push_str("FORW ");
    }
    if has_all_flags(flags, SYM_DEF) {
        s.push_str("DEF ");
    }
    if has_all_flags(flags, SYM_EQ) {
        s.push_str("EQ ");
    }
    if has_all_flags(flags, SYM_LBL) {
        s.push_str("LBL ");
    }
    if has_all_flags(flags, SYM_REG) {
        s.push_str("REG ");
    }
    if has_all_flags(flags, SYM_PRE) {
        s.push_str("PRE ");
    }
    if has_all_flags(flags, SYM_XTV) {
        s.push_str("XTV ");
    }
    if has_all_flags(flags, SYM_MUL) {
        s.push_str("MUL ");
    }
    if has_all_flags(flags, SYM_RPT) {
        s.push_str("RPT ");
    }
    if has_all_flags(flags, SYM_GLB) {
        s.push_str("GLB ");
    }
    if has_all_flags(flags, SYM_SML) {
        s.push_str("SML ");
    }
    if has_all_flags(flags, SYM_ADJ) {
        s.push_str("ADJ ");
    }
    if has_all_flags(flags, SYM_DISC) {
        s.push_str("DISC ");
    }
    if has_all_flags(flags, SYM_LIT) {
        s.push_str("LIT ");
    }
    if !has_any_flags(flags, SYM_DEF | SYM_LIT) {
        s.push_str("UNDEF ");
    }
    if has_all_flags(flags, SYM_DEF) {
        s.push_str("RELOC ");
    }
    if has_all_flags(flags, SYM_GLB) && !has_any_flags(flags, SYM_DEF) {
        s.push_str("EXTERN ");
    }
    if has_all_flags(flags, SYM_BASE) {
        s.push_str("BASE ");
    }
    if s.len() > 0 {
        s.pop();
    }
    s
}
