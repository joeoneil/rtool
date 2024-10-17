use std::fs;

use clap::Args;

use crate::common::ObjectModule;

#[derive(Args, Clone)]
#[command(about = "Dump the contents of one or more object modules. 
If no flags are specified, prints all information about all files
")]
pub struct DumpArgs {
    #[arg(short = 'd', help = "Dump the contents of the data section")]
    data: bool,
    #[arg(short = 'f', help = "Dump the contents of the reference list")]
    reference: bool,
    #[arg(short = 'l', help = "Dump the contents of the relocation list")]
    relocation: bool,
    #[arg(
        short = 'm',
        help = "Dump the contents of the module table (if present)"
    )]
    modtab: bool,
    #[arg(short = 'r', help = "Dump the contents of the rdata section")]
    rdata: bool,
    #[arg(short = 's', help = "Dump the contents of the sdata section")]
    sdata: bool,
    #[arg(short = 't', help = "Dump the contents of the text section")]
    text: bool,
    #[arg(short = 'y', help = "Dump the contents of the symbol table")]
    symtab: bool,
    files: Vec<String>,
}

pub fn dump(args: &DumpArgs) {
    // if no flags specified, print everything
    let all = !(args.data
        || args.reference
        || args.relocation
        || args.modtab
        || args.rdata
        || args.sdata
        || args.text
        || args.symtab);
    let oms = args
        .files
        .iter()
        .map(|f| fs::read(f).expect(format!("Failed to read file {}", f).as_str()))
        .map(|v| ObjectModule::from_slice_u8(v.as_slice()).expect("Failed to parse object module"))
        .collect::<Vec<_>>();

    for om in oms {
        println!("{}", om.head);
        if all || args.text {
            om.print_sect("text", om.text.as_slice());
        }
        if all || args.rdata {
            om.print_sect("rdata", om.rdata.as_slice());
        }
        if all || args.data {
            om.print_sect("data", om.data.as_slice());
        }
        if all || args.sdata {
            om.print_sect("sdata", om.sdata.as_slice());
        }
        if all || args.relocation {
            om.print_rel();
        }
        if all || args.reference {
            om.print_ref();
        }
        if all || args.symtab {
            om.print_sym();
        }
    }
}
