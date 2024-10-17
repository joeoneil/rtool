use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    fs::{self, File},
    io::{Read, Write},
    mem::transmute,
    os::unix::fs::OpenOptionsExt,
};

use clap::Args;
use lazy_static::lazy_static;

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

#[derive(Args, Clone)]
pub struct SimArgs {
    #[arg(
        short = 'a',
        help = "Print the interpreted address (in addition to the numerical address) for every
instruction when instruction tracing is being done. (By default, only \"exact\"
addresses - those which exactly match the address associated with a specific 
symbol in the symbol table - are printed.)"
    )]
    interp_address: bool,
    #[arg(
        short = 'b',
        help = "Initialize the BSS and SBSS segment contents to the value specified by 
N instead of the default value of 0. See below for a description of how
this value is specified.",
        default_value_t = 0
    )]
    bss_val: u8,
    #[arg(
        short = 'c',
        help = "Print the machine instruction word along with the decoded instruction when 
instruction tracing is being done. (By default, only the decoded instruction 
is printed.)"
    )]
    print_machine: bool,
    #[arg(
        short = 'd',
        help = "Use the rsim debugger, rbug (described later in this document)."
    )]
    debug: bool,
    #[arg(
        short = 'e',
        help = "Do not copy environment variable strings or the environment vector onto the
runtime stack at the start of execution. This reduces the amount of information
pushed onto the stack before the program begins to execute. (Normally, the
environment strings are placed on the stack first, followed by the argument
strings, environment vector, and argument vector.)"
    )]
    no_env: bool,
    #[arg(
        short = 'f',
        help = "Force a dump of register and memory contents after the termination of the 
simulated program regardless of the termination status."
    )]
    force_dump: bool,
    #[arg(
        short = 'H',
        help = "Set the initial size of the runtime heap to NKB (i.e., N * 1024 bytes).",
        default_value_t = 0
    )]
    heap_size: u32,
    #[arg(
        short = 'i',
        help = "Allow a maximum of N instructions to be executed. This allows the user to 
place a runtime limit on the simulation of programs. The default is to allow
an infinite number of instructions; the minimum allowed is one instruction.",
        default_value_t = 0
    )]
    max_inst: u32,
    #[arg(
        short = 'k',
        help = "Normally, rsim will randomly change the contents of the \"kernel registers\" 
($k0 and $k1) during simulated execution, as would happen naturally when the 
operating system performs exception handling. This option disables that."
    )]
    no_kern_clobber: bool,
    #[arg(
        short = 'l',
        help = "Use long output lines. Output from memory and register dumps will be written
assuming 136-column lines. The default assumes 80-column lines."
    )]
    long_lines: bool,
    #[arg(
        short = 'm',
        help = "Cause a dump of register and memory contents if the simulated program 
terminates abnormally (e.g., through a memory fault)."
    )]
    error_dump: bool,
    #[arg(
        short = 'n',
        help = "Use register numbers in register dumps. The default is to use their names
rather than their numbers."
    )]
    reg_nums: bool,
    #[arg(
        short = 'p',
        help = "Print statistics on the number of instructions executed at the end of the
simulation."
    )]
    inst_stats: bool,
    #[arg(
        short = 's',
        help = "Use an initial runtime stack size of NKB (N * 1024 bytes). The default is 8KB;
the minimum allowed is 1KB. The size will be rounded up (if needed) to a
multiple of eight.",
        default_value_t = 8
    )]
    stack_size: u32,
    #[arg(
        short = 't',
        help = "Turn on instruction tracing. Each instruction will be printed (in decoded
form) prior to its simulated execution."
    )]
    trace: bool,
    #[arg(
        short = 'x',
        help = "Force execution regardless of the mode of the load module. Normally, if the
load module is not marked as executable (e.g., because it was not completely
linked), rsim will refuse to execute it. "
    )]
    force_exec: bool,
    file: String,
    program_args: Vec<String>,
}

lazy_static! {
    static ref EMPTY_ARGS: SimArgs = SimArgs::empty();
}

impl SimArgs {
    fn empty() -> Self {
        SimArgs {
            interp_address: false,
            bss_val: 0,
            print_machine: false,
            debug: false,
            no_env: false,
            force_dump: false,
            heap_size: 0,
            max_inst: 0,
            no_kern_clobber: false,
            long_lines: false,
            error_dump: false,
            reg_nums: false,
            inst_stats: false,
            stack_size: 8,
            trace: false,
            force_exec: false,
            file: String::new(),
            program_args: vec![],
        }
    }
}

pub fn sim(args: &SimArgs) {
    let om = ObjectModule::from_slice_u8(
        fs::read(args.file.as_str())
            .expect("Failed to read object module file")
            .as_slice(),
    )
    .expect("Invalid object module file");

    let exec = Exec::new(om, args).expect("");

    if !args.debug {
        let e = exec.run().unwrap_err();
    } else {
        todo!("Debugger not implemented");
    }
}
