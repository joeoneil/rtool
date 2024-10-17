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

pub fn flag_name(flag: u32) -> &'static str {
    if flag & SYM_FORW > 0 {
        "FORW"
    } else if has_any_flags(flag, SYM_DEF) {
        "DEF"
    } else if has_any_flags(flag, SYM_EQ) {
        "EQ"
    } else if has_any_flags(flag, SYM_LBL) {
        "LBL"
    } else if has_any_flags(flag, SYM_REG) {
        "REG"
    } else if has_any_flags(flag, SYM_PRE) {
        "PRE"
    } else if has_any_flags(flag, SYM_UNDEF) {
        "UNDEF"
    } else if has_any_flags(flag, SYM_XTV) {
        "XTV"
    } else if has_any_flags(flag, SYM_MUL) {
        "MUL"
    } else if has_any_flags(flag, SYM_RPT) {
        "RPT"
    } else if has_any_flags(flag, SYM_GLB) {
        "GLB"
    } else if has_any_flags(flag, SYM_SML) {
        "SML"
    } else if has_any_flags(flag, SYM_ADJ) {
        "ADJ"
    } else if has_any_flags(flag, SYM_DISC) {
        "DISC"
    } else if has_any_flags(flag, SYM_LIT) {
        "LIT"
    } else {
        ""
    }
}

pub fn flags_string(flags: u32) -> String {
    let mut s = String::new();
    for i in (4..=18) {
        if flags & (1 << i) > 0 {
            s.push_str(flag_name(1 << i));
            s.push(' ')
        }
    }
    if has_all_flags(flags, SYM_EQ | SYM_LIT) {
        s.push_str("CONST");
        s.push(' ');
    }
    if !has_any_flags(flags, SYM_DEF | SYM_LIT) {
        if !has_any_flags(flags, SYM_UNDEF) {
            s.push_str("UNDEF");
            s.push(' ');
        }
        if has_any_flags(flags, SYM_GLB) {
            s.push_str("EXTERN");
            s.push(' ');
        }
    }
    s
}
