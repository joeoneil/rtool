use lazy_static::lazy_static;

use clap::Args;

use crate::common::*;

mod linker;

lazy_static! {
    #[rustfmt::skip]
    pub static ref r2k_startup_obj: ObjectModule = ObjectModule {
        head: ObjectHeader {
            magic: 0xface,
            version: 0x2cc6,
            flags: 0x00000000,
            entry: 0x00000000,
            data: [
                0x00000044, 0x00000000, 0x00000008, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000003, 0x00000006, 0x0000003D,
            ]
        },
        text: vec![
            0x00, 0x0b, 0xad, 0x0d, // break 0x2eb4
            0x34, 0x02, 0x00, 0x09, // ori $v0, $zero, 0x0009     # brk 0 - requests info about
            0x34, 0x04, 0x00, 0x00, // ori $a0, $zero, 0x0000     # the heap without allocating
            0x00, 0x00, 0x00, 0x0c, // syscall
            0x3c, 0x01, 0x00, 0x00, // lui $at, 0x0000
            0xac, 0x22, 0x00, 0x00, // sw $v0, 0x0000($at)        # These addresses get filled in
            0x3c, 0x01, 0x00, 0x00, // lui $at, 0x0000            # at link time
            0xac, 0x23, 0x00, 0x00, // sw $v1, 0x0000($at)
            0x8f, 0xa4, 0x00, 0x00, // lw $a0, $sp, 0x0000        # argc
            0x8f, 0xa5, 0x00, 0x04, // lw $a1, $sp, 0x0004        # argv
            0x8f, 0xa6, 0x00, 0x08, // lw $a3, $sp, 0x0008        # envp
            0x0c, 0x00, 0x00, 0x00, // jal 0x00000000             # jal main (extern)
            0x00, 0x00, 0x00, 0x00, // nop                        # nop (added by assembler)
            0x00, 0x40, 0x20, 0x20, // add $a0, $v0, $zero
            0x34, 0x02, 0x00, 0x11, // ori $v0, $zero, 0x0011     # SYS_EXIT2
            0x00, 0x00, 0x00, 0x0c, // syscall
            0x00, 0x00, 0x00, 0x00, // nop                        # nop (alignment)
        ],
        rdata: vec![],
        data: vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        sdata: vec![],
        rel_info: vec![],
        ext_ref: vec![
            RefEntry {
                addr: 0x00000010,
                str_off: 0x00000033,
                ref_info: RefInfo {
                    ix: 0x0000,
                    unknown: RefUnknown::PLUS,
                    typ: RefType::IMM2,
                    sect: Location::TEXT,
                }
            },
            RefEntry {
                addr: 0x00000018,
                str_off: 0x00000005,
                ref_info: RefInfo {
                    ix: 0x0000,
                    unknown: RefUnknown::PLUS,
                    typ: RefType::IMM2,
                    sect: Location::TEXT,
                }
            },
            RefEntry {
                addr: 0x0000002c,
                str_off: 0x00000000,
                ref_info: RefInfo {
                    ix: 0x0000,
                    unknown: RefUnknown::PLUS,
                    typ: RefType::JUMP,
                    sect: Location::TEXT,
                }
            }
        ],
        symtab: vec![
            SymEntry {
                flags: Location::TEXT as u32 | SYM_GLB | SYM_LBL,
                val: 0x00000000,
                str_off: 0x00000000, // main
                ofid: 0x00,
            },
            SymEntry {
                flags: Location::DATA as u32 | SYM_GLB | SYM_LBL | SYM_DEF | SYM_FORW,
                val: 0x00000004,
                str_off: 0x00000005, // __heap_size
                ofid: 0x00,
            },
            SymEntry {
                flags: Location::ABS as u32 | SYM_DEF | SYM_EQ,
                val: 0x00000011,
                str_off: 0x00000011, // SYS_EXIT2
                ofid: 0x00,
            },
            SymEntry {
                flags: Location::ABS as u32 | SYM_DEF | SYM_EQ,
                val: 0x00000011,
                str_off: 0x0000001b, // SYS_SBRK
                ofid: 0x00,
            },
            SymEntry {
                flags: Location::TEXT as u32 | SYM_FORW | SYM_DEF | SYM_LBL | SYM_GLB,
                val: 0x00000000,
                str_off: 0x00000024, // __r2k__entry__
                ofid: 0x00,
            },
            SymEntry {
                flags: Location::DATA as u32 | SYM_FORW | SYM_DEF | SYM_LBL | SYM_GLB | SYM_FORW,
                val: 0x00000000,
                str_off: 0x00000033, // __heap_ptr
                ofid: 0x00,
            }
        ],
        strtab: vec![
            "main", // 0x00
            "__heap_size", // 0x05
            "SYS_EXIT2", // 0x11
            "SYS_SBRK", // 0x1b
            "__r2k__entry__", // 0x24
            "__heap_ptr" // 0x33
        ].into_iter()
        .flat_map(|s| s.chars().map(|c| c as u8).chain([0].into_iter()))
        .collect(),
    };
}

#[derive(Args, Clone)]
#[command(
    about = "Link one or more object modules produced by rasm or rlink into one executable
"
)]
pub struct LinkerArgs {
    #[arg(
        short = 'm',
        help = "Print a load map showing the relocated addresses of all symbols defined in the object modules being linked."
    )]
    load_map: bool,
    #[arg(
        short = 'o',
        help = "Use this as the name of the load module to be created. The default name is determined by the object module which contains the entry point main; if none is found and no -o option is given, r.out is used."
    )]
    out: Option<String>,
    #[arg(
        short = 's',
        help = "Use the specified file as the startup routine. By default an internal object is used"
    )]
    startup: Option<String>,
    files: Vec<String>,
}

pub fn link(args: &LinkerArgs) {
    let mut objs = vec![];

    for f in &args.files {
        objs.push(
            ObjectModule::from_slice_u8(
                &std::fs::read(f).expect(format!("Could not read file {}", f).as_str()),
            )
            .expect(format!("Invalid object module {}", f).as_str()),
        );
    }

    if let Some(f) = &args.startup {
        let start = ObjectModule::from_slice_u8(
            &std::fs::read(f).expect(format!("Could not read file {}", f).as_str()),
        )
        .expect(format!("Invalid object module {}", f).as_str());
        objs.push(start);
    } else {
        objs.push(r2k_startup_obj.clone());
    }

    let out = crate::link::linker::link(objs, args);
}
