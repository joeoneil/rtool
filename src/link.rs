use lazy_static::lazy_static;

use clap::Args;

use crate::common::{ObjectHeader, ObjectModule, RefEntry, RelEntry, SymEntry};

lazy_static! {
    static ref r2k_startup_obj: ObjectModule = ObjectModule {
        head: ObjectHeader {
            magic: 0xface,
            version: 0x2cc6,
            flags: 0x00000000,
            entry: 0x00000000,
            data: [
                0x00000000, 0x00000000, 0x00000008, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
                0x00000000, 0x00000000, 0x00000000,
            ]
        },
        text: vec![],
        rdata: vec![],
        data: vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        sdata: vec![],
        rel_info: vec![],
        ext_ref: vec![],
        symtab: vec![],
        strtab: vec![],
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
    todo!();
}
