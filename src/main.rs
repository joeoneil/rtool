use std::fs;

fn main() {
    /*
    let in_file = fs::read_to_string("./src/examples/asm/colony.asm").unwrap();
    rtool::asm::dbg_parse(in_file, Rule::program).unwrap();
    */

    let in_obj = fs::read("./data/Homeworks/hw1/sum.out").unwrap();
    let obj = rtool::common::ObjectModule::from_slice_u8(in_obj.as_slice()).unwrap();
    let exec = rtool::sim::Exec::new(obj).unwrap();
    exec.run();
}
