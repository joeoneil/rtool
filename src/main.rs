use std::fs;

use clap::{Parser, Subcommand};

use rtool::{
    dump::{dump, DumpArgs},
    link::{link, LinkerArgs},
    sim::{sim, SimArgs},
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    Dump(DumpArgs),
    Link(LinkerArgs),
    Run(SimArgs),
}

fn main() {
    fs::write(
        "./dump.obj",
        // rtool::common::module::obj.clone().to_vec_u8().as_slice(),
        rtool::link::r2k_startup_obj.clone().to_vec_u8().as_slice(),
    )
    .unwrap();

    let cli = Cli::parse();

    match cli.command {
        Commands::Dump(args) => dump(&args),
        Commands::Link(args) => link(&args),
        Commands::Run(args) => sim(&args),
    }

    /*
    let in_file = fs::read_to_string("./src/examples/asm/colony.asm").unwrap();
    rtool::asm::dbg_parse(in_file, Rule::program).unwrap();
    */

    /*
    let in_obj = fs::read("./data/Projects/proj1/colony.out").unwrap();
    let obj = rtool::common::ObjectModule::from_slice_u8(in_obj.as_slice()).unwrap();
    let exec = rtool::sim::Exec::new(obj).unwrap();
    let _ = exec.run().unwrap_err();
    */
}
