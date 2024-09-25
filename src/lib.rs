#![allow(unused)]

/// Assembler functionality. Parses a given source file and produces a MIPS
/// object file, which can then be linked into an executable with rlink.
pub mod asm;
/// Common functionality between 2 or more modules. Includes utilities for
/// reading and writing object and executable files
pub mod common;
/// Dumps symbols and section information from a given object or executable
pub mod dump;
/// Links multiple object files into an executable
pub mod link;
/// Functionality for simulating a MIPS CPU, running the provided executable
pub mod sim;
