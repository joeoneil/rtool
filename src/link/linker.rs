
use std::collections::HashMap;
use std::ffi::CString;

use itertools::Itertools;

use crate::common::Location;
use crate::link::{ObjectModule, ObjectHeader, LinkerArgs};

#[derive(Copy, Clone, Debug)]
struct OMLinkInfo {
    ofid: u16,
    sect_off: [u32; 10],
}

pub fn link(obj: Vec<ObjectModule>, args: &LinkerArgs) -> ObjectModule {
    
    let mut info = OMLinkInfo {
        ofid: 0,
        sect_off: [0; 10],
    };

    let bin_sections = [
        Location::TEXT,
        Location::RDATA,
        Location::DATA,
        Location::SDATA,
        Location::SBSS,
        Location::BSS,
        Location::STR,
    ].into_iter().map(|l| l as usize).collect::<Vec<_>>();

    // associate each object with section offsets (binary sections only)
    let obj = obj.into_iter()
        .map(|o| {
            let out = (o.clone(), info.clone());
            info.ofid += 1;
            for loc in &bin_sections {
                info.sect_off[*loc] += o.head.data[*loc];
                if *loc != Location::STR as usize {
                    if *loc == Location::TEXT as usize {
                        align_to(&mut info.sect_off[*loc], 4);
                    } else {
                        align_to(&mut info.sect_off[*loc], 8);
                    }
                }
            }
            out
        }).collect::<Vec<_>>();

    for (_, info) in &obj {
        println!("{}: {:?}", info.ofid, info);
    }
    
    let mut out = ObjectModule {
        head: ObjectHeader {
            magic: 0xface,
            version: 0x2cc6,
            flags: 0x00000000,
            entry: 0x00000000,
            data: info.sect_off,
        },
        text: vec![0; info.sect_off[Location::TEXT as usize] as usize],
        rdata: vec![0; info.sect_off[Location::RDATA as usize] as usize],
        data: vec![0; info.sect_off[Location::DATA as usize] as usize],
        sdata: vec![0; info.sect_off[Location::SDATA as usize] as usize],
        rel_info: vec![],
        ext_ref: vec![],
        symtab: vec![],
        strtab: vec![0; info.sect_off[Location::STR as usize] as usize],
    };
    
    // actually concatenate binary sections
    for (om, info) in &obj {
        for idx in 0..om.text.len() {
            out.text[idx + info.sect_off[Location::TEXT as usize] as usize] = om.text[idx];
        }
        for idx in 0..om.rdata.len() {
            out.rdata[idx + info.sect_off[Location::RDATA as usize] as usize] = om.rdata[idx];
        }
        for idx in 0..om.data.len() {
            out.rdata[idx + info.sect_off[Location::DATA as usize] as usize] = om.data[idx];
        }
        for idx in 0..om.sdata.len() {
            out.sdata[idx + info.sect_off[Location::SDATA as usize] as usize] = om.sdata[idx];
        }
        for idx in 0..om.strtab.len() {
            out.strtab[idx + info.sect_off[Location::STR as usize] as usize] = om.strtab[idx];
        }
    }

    let string_map = string_dedup(&mut out);

    fn map_string(idx: u32) -> u32 {
        string_map.get(&idx).expect("panic: failed get string idx");
    }

    fn map_string_ofid(ofid: usize, idx: u32) -> u32 {
        map_string(obj[ofid].1.sect_off[Location::STR as usize])
    }

    

    todo!();
}

fn align_to(val: &mut u32, align: u32) {
    *val = ((*val + align - 1) / align) * align;
}

// deduplicate strings in an object module, preserving order
// returns a mapping from previous index to new index.
fn string_dedup(obj: &mut ObjectModule) -> HashMap<u32, u32> {
    let mut addr_str: HashMap<u32, CString> = HashMap::new();

    let mut addr = 0;
    while addr < obj.strtab.len() {
        if let Some(entry) = obj.get_str_entry(addr) {
            let len = entry.count_bytes() + 1;
            addr_str.insert(addr as u32, entry);
            addr += len;
        } else {
            panic!("panic: failed to read string at addr: {:08x}", addr);
        }
    }

    // println!("{:?}", addr_str);

    let mut str_addr: HashMap<CString, u32> = HashMap::new();
    let mut addr_map: HashMap<u32, u32> = HashMap::new();

    let mut new_bytes = vec![];
    
    for (addr, s) in addr_str.into_iter().sorted_by(|(a1, _), (a2, _)| a1.cmp(a2)) {
        if let Some(mapped_addr) = str_addr.get(&s) {
            addr_map.insert(addr, *mapped_addr);
        } else {
            let mapped_addr = new_bytes.len();
            addr_map.insert(addr, mapped_addr as u32);
            let mut bytes = s.as_bytes_with_nul().into_iter().copied().collect();
            new_bytes.append(&mut bytes);
            str_addr.insert(s, mapped_addr as u32);
        }
    }

    obj.strtab = new_bytes;

    addr_map
}



















