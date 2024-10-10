use std::{
    ffi::CString,
    fmt::{Debug, Display},
    num::NonZeroU32,
};

use lazy_static::lazy_static;

use super::{
    types::{ObjectHeader, ObjectModule},
    Location, RefInfo, RefUnknown, SymEntry,
};
use crate::common::{RefEntry, RefType, RelEntry};

lazy_static! {
    pub static ref obj: ObjectModule = ObjectModule {
        head: ObjectHeader {
            magic: 0xface,
            version: 0x2cc6,
            flags: 0x00000000,
            entry: 0x00000000,
            data: [
                0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
                0x00000000, 0x00000020, 0x00005000,
            ]
        },
        text: vec![],
        rdata: vec![],
        data: vec![],
        sdata: vec![],
        rel_info: vec![],
        ext_ref: vec![],
        symtab: (0..0x20)
            .into_iter()
            .map(|i| SymEntry {
                flags: 1 << i,
                str_off: (i * 5),
                val: 0,
                ofid: 0,
            })
            .collect(),
        strtab: (0..0x1000)
            .flat_map(|e| format!("{:04x}", e).bytes().chain([0]).collect::<Vec<_>>())
            .collect(),
    };
}

impl ObjectHeader {
    pub fn from_slice_u8(data: &[u8]) -> Option<Self> {
        if data.len() != 52 {
            return None;
        }

        if data[0..2] != u16::to_be_bytes(0xface) {
            return None;
        }

        let mut sizes = [0; 10];

        for i in 0..10 {
            sizes[i] = u32::from_be_bytes(data[(12 + 4 * i)..(16 + 4 * i)].try_into().unwrap())
        }

        Some(Self {
            magic: u16::from_be_bytes(data[0..2].try_into().unwrap()),
            version: u16::from_be_bytes(data[2..4].try_into().unwrap()),
            flags: u32::from_be_bytes(data[4..8].try_into().unwrap()),
            entry: u32::from_be_bytes(data[8..12].try_into().unwrap()),
            data: sizes,
        })
    }

    pub fn to_vec_u8(self) -> Vec<u8> {
        let mut buf = vec![];
        buf.extend_from_slice(&self.magic.to_be_bytes());
        buf.extend_from_slice(&self.version.to_be_bytes());
        buf.extend_from_slice(&self.flags.to_be_bytes());
        buf.extend_from_slice(&self.entry.to_be_bytes());
        for d in self.data {
            buf.extend_from_slice(&d.to_be_bytes());
        }
        buf
    }
}

impl Debug for ObjectHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entry = NonZeroU32::new(self.entry);
        write!(
            f,
            "ObjectHeader {{ magic: 0x{:x}, version: 0x{:x}, flags: 0b{:b}, entry: {}, data: [text: {}, rdata: {}, data: {}, sdata: {}, sbss: {}, bss: {}, relinfo: {}, reflist: {}, symtab: {}, strtab: {}] }}",
            self.magic, self.version, self.flags, entry.map(|i| format!("0x{:x}", i)).unwrap_or(String::from("None")), self.data[0], self.data[1], self.data[2], self.data[3], self.data[4], self.data[5], self.data[6], self.data[7], self.data[8], self.data[9],
        )
    }
}

impl Display for ObjectHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "magic: {:x} version: {:x} flags: {:08x} entry point: {}",
            self.magic,
            self.version,
            self.flags,
            NonZeroU32::new(self.entry)
                .map(|e| format!("{:08x}", e))
                .unwrap_or(String::from("None"))
        )?;
        writeln!(
            f,
            "sizes (bytes): text {} rdata {} data {} sdata {} sbss {} bss {} strs {}",
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
            self.data[4],
            self.data[5],
            self.data[9]
        )?;
        writeln!(
            f,
            "counts: rel {} ref {} syms {}",
            self.data[6], self.data[7], self.data[8],
        )
    }
}

impl ObjectModule {
    pub fn from_slice_u8(data: &[u8]) -> Result<Self, String> {
        let head = ObjectHeader::from_slice_u8(&data[..52])
            .ok_or(String::from("Failed to parse header"))?;
        let mut bytes = data.into_iter().skip(52);
        let text = bytes
            .by_ref()
            .take(head.data[0] as usize)
            .copied()
            .collect::<Vec<_>>();
        let rdata = bytes
            .by_ref()
            .take(head.data[1] as usize)
            .copied()
            .collect::<Vec<_>>();
        let data = bytes
            .by_ref()
            .take(head.data[2] as usize)
            .copied()
            .collect::<Vec<_>>();
        let sdata = bytes
            .by_ref()
            .take(head.data[3] as usize)
            .copied()
            .collect::<Vec<_>>();

        let mut rel_info: Vec<RelEntry> = vec![];
        for _ in 0..head.data[6] {
            let rel_bytes: [u8; 8] = bytes
                .by_ref()
                .take(8)
                .copied()
                .collect::<Vec<_>>()
                .as_slice()
                .try_into()
                .map_err(|_| String::from("Reached end of data while parsing rel info"))?;
            rel_info.push(RelEntry::from_bytes(rel_bytes).expect("Invalid relocation entry"));
        }

        let mut ext_ref: Vec<RefEntry> = vec![];
        for _ in 0..head.data[7] {
            let ref_bytes: [u8; 12] = bytes
                .by_ref()
                .take(12)
                .copied()
                .collect::<Vec<_>>()
                .as_slice()
                .try_into()
                .map_err(|_| String::from("Reached end of data while parsing ref info"))?;
            ext_ref.push(RefEntry::from_bytes(ref_bytes).expect("Invalid reference entry"));
        }

        let mut symtab: Vec<SymEntry> = vec![];
        for _ in 0..head.data[8] {
            let sym_bytes: [u8; 16] = bytes
                .by_ref()
                .take(16)
                .copied()
                .collect::<Vec<_>>()
                .as_slice()
                .try_into()
                .map_err(|_| String::from("Reached end of data while parsing symbol table"))?;
            symtab.push(SymEntry::from_bytes(sym_bytes).expect("Invalid symtab entry"));
            /*
            let flags = u32::from_be_bytes(sym_bytes[0..4].try_into().unwrap());
            let val = u32::from_be_bytes(sym_bytes[4..8].try_into().unwrap());
            let str_off = u32::from_be_bytes(sym_bytes[8..12].try_into().unwrap());
            let ofid = u16::from_be_bytes(sym_bytes[12..14].try_into().unwrap());
            symtab.push(SymEntry {
                val,
                flags,
                str_off,
                ofid,
            });
            */
            /*
            println!(
                "raw sym: {:02x}{:02x}{:02x}{:02x} {:02x}{:02x}{:02x}{:02x} {:02x}{:02x}{:02x}{:02x} {:02x}{:02x}{:02x}{:02x}",
                sym_bytes[0],
                sym_bytes[1],
                sym_bytes[2],
                sym_bytes[3],
                sym_bytes[4],
                sym_bytes[5],
                sym_bytes[6],
                sym_bytes[7],
                sym_bytes[8],
                sym_bytes[9],
                sym_bytes[10],
                sym_bytes[11],
                sym_bytes[12],
                sym_bytes[13],
                sym_bytes[14],
                sym_bytes[15]
            )
            */
        }

        let strtab: Vec<u8> = bytes
            .by_ref()
            .take(head.data[9] as usize)
            .copied()
            .collect();

        if strtab.len() != head.data[9] as usize {
            return Err(String::from(
                "Reached end of data while reading string table",
            ));
        }

        // TODO: mod tab
        // println!("Remaining bytes in object file: {}", bytes.count());

        Ok(ObjectModule {
            head,
            text,
            rdata,
            data,
            sdata,
            rel_info,
            ext_ref,
            symtab,
            strtab,
        })
    }

    fn print_sect(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        sect: &str,
        data: &[u8],
    ) -> std::fmt::Result {
        if data.len() > 0 {
            write!(f, "sect: {} ({} bytes)\n ", sect, data.len())?;
            let mut chunk_counter = 0;
            let mut line_counter = 0;
            for b in data {
                write!(f, "{:02x}", b)?;
                chunk_counter += 1;
                if chunk_counter == 4 {
                    chunk_counter = 0;
                    line_counter += 1;
                    write!(f, " ")?;
                }
                if line_counter == 8 {
                    line_counter = 0;
                    write!(f, "\n ")?;
                }
            }
            if line_counter != 0 || chunk_counter != 0 {
                write!(f, "\n")?;
            } else {
                write!(f, "\r")?;
            }
        }
        Ok(())
    }

    fn print_rel(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.rel_info.len() > 0 {
            writeln!(f, "relocation: {} entries", self.rel_info.len())?;
            for rel in &self.rel_info {
                writeln!(
                    f,
                    " rel: addr {:08x} {} {}",
                    rel.addr,
                    // Other sections cannot be relocatable (maybe?)
                    match rel.sect {
                        Location::TEXT => "TEXT",
                        Location::RDATA => "RDATA",
                        Location::DATA => "DATA",
                        Location::SDATA => "SDATA",
                        s => panic!("Invalid relocation section {}", s as u8),
                    },
                    match rel.rel_info {
                        RefType::IMM => "IMM",
                        RefType::IMM2 => "IMM2",
                        RefType::IMM3 => "IMM3",
                        RefType::WORD => "WORD",
                        RefType::JUMP => "JUMP",
                    }
                );
            }
        }

        Ok(())
    }

    fn print_ref(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.ext_ref.len() > 0 {
            writeln!(f, "references: {} entries", self.ext_ref.len())?;
            for r in &self.ext_ref {
                writeln!(
                    f,
                    " ref: addr {:08x} sym {:?} ix {:04x} {} + {}",
                    r.addr,
                    self.get_str_entry(r.str_off as usize)
                        .expect(format!("Invalid reftab entry offset {}", r.str_off).as_str()),
                    r.ref_info.ix,
                    match r.ref_info.typ {
                        RefType::IMM => "IMM",
                        RefType::IMM2 => "IMM2",
                        RefType::IMM3 => "IMM3",
                        RefType::JUMP => "JUMP",
                        RefType::WORD => "WORD",
                    },
                    match r.ref_info.sect {
                        Location::TEXT => "TEXT",
                        Location::DATA => "DATA",
                        Location::RDATA => "RDATA",
                        Location::SDATA => "SDATA",
                        _ => unreachable!(),
                    }
                )?;
            }
        }
        Ok(())
    }

    fn print_sym(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }

    pub fn get_str_entry(&self, offset: usize) -> Option<CString> {
        // check that string is the first string or immediately follows a NUL byte
        if offset != 0
            && self
                .strtab
                .get((offset - 1) as usize)
                .is_some_and(|c| *c != 0)
        {
            return None;
        }
        let buf = self
            .strtab
            .iter()
            .skip(offset)
            .take_while(|b| **b != 0)
            .copied()
            .collect::<Vec<_>>();
        CString::new(buf).ok()
    }

    pub fn to_vec_u8(self) -> Vec<u8> {
        let mut buf = vec![];
        buf.extend_from_slice(self.head.to_vec_u8().as_slice());
        buf.extend_from_slice(self.text.as_slice());
        buf.extend_from_slice(self.rdata.as_slice());
        buf.extend_from_slice(self.data.as_slice());
        buf.extend_from_slice(self.sdata.as_slice());
        for rel in self.rel_info {
            buf.extend_from_slice(&rel.to_bytes());
        }
        for e_ref in self.ext_ref {
            buf.extend_from_slice(&e_ref.to_bytes());
        }
        for sym in self.symtab {
            buf.extend_from_slice(&sym.to_bytes());
        }
        buf.extend_from_slice(self.strtab.as_slice());
        buf
    }
}

impl Display for ObjectModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.head)?;
        self.print_sect(f, "text", self.text.as_slice())?;
        self.print_sect(f, "rdata", self.rdata.as_slice())?;
        self.print_sect(f, "data", self.data.as_slice())?;
        self.print_sect(f, "sdata", self.sdata.as_slice())?;
        self.print_rel(f)?;
        self.print_ref(f)?;
        self.print_sym(f)?;
        Ok(())
    }
}

impl RelEntry {
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut buf = [0; 8];

        let a_bytes = self.addr.to_be_bytes();
        for i in 0..4 {
            buf[i] = a_bytes[i];
        }
        buf[4] = self.sect as u8;
        buf[7] = self.rel_info as u8;

        buf
    }

    pub fn from_bytes(bytes: [u8; 8]) -> Option<Self> {
        let addr = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let sect = bytes[4].try_into().ok()?;
        let rel_info = bytes[7].try_into().ok()?;

        Some(Self {
            addr,
            sect,
            rel_info,
        })
    }
}

impl RefEntry {
    pub fn to_bytes(&self) -> [u8; 12] {
        let mut buf = [0; 12];

        let a_bytes = self.addr.to_be_bytes();
        for i in 0..4 {
            buf[i] = a_bytes[i];
        }

        let off_bytes = self.str_off.to_be_bytes();
        for i in 0..4 {
            buf[i + 4] = off_bytes[i];
        }

        let ix_bytes = self.ref_info.ix.to_be_bytes();
        for i in 0..2 {
            buf[i + 8] = ix_bytes[i];
        }

        buf[10] = ((self.ref_info.unknown as u8) << 4) | (self.ref_info.typ as u8);
        buf[11] = self.ref_info.sect as u8;

        buf
    }

    pub fn from_bytes(bytes: [u8; 12]) -> Option<Self> {
        let addr = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let str_off = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        let ix = u16::from_be_bytes(bytes[8..10].try_into().unwrap());
        let unknown = RefUnknown::try_from(bytes[10] >> 4).ok()?;
        let typ = RefType::try_from(bytes[10] & 0x0F).ok()?;
        let sect = Location::try_from(bytes[11]).ok()?;

        Some(Self {
            addr,
            str_off,
            ref_info: RefInfo {
                ix,
                unknown,
                typ,
                sect,
            },
        })
    }
}

impl SymEntry {
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut buf = [0; 16];

        let f_bytes = self.flags.to_be_bytes();
        for i in 0..4 {
            buf[i] = f_bytes[i];
        }

        let v_bytes = self.val.to_be_bytes();
        for i in 0..4 {
            buf[i + 4] = v_bytes[i];
        }

        let s_bytes = self.str_off.to_be_bytes();
        for i in 0..4 {
            buf[i + 8] = s_bytes[i];
        }

        let o_bytes = self.ofid.to_be_bytes();
        for i in 0..2 {
            buf[i + 12] = o_bytes[i];
        }

        buf
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Option<Self> {
        let flags = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let val = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        let str_off = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        let ofid = u16::from_be_bytes(bytes[12..14].try_into().unwrap());

        Some(Self {
            flags,
            val,
            str_off,
            ofid,
        })
    }

    #[inline]
    pub fn has_flags(&self, flags: u32) -> bool {
        self.flags & flags == flags
    }

    #[inline]
    pub fn has_any_flag(&self, flags: u32) -> bool {
        self.flags & flags > 0
    }
}

impl TryFrom<u8> for Location {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::TEXT),
            1 => Ok(Self::RDATA),
            2 => Ok(Self::DATA),
            3 => Ok(Self::SDATA),
            4 => Ok(Self::SBSS),
            5 => Ok(Self::BSS),
            6 => Ok(Self::REL),
            7 => Ok(Self::REF),
            8 => Ok(Self::SYM),
            9 => Ok(Self::STR),
            10 => Ok(Self::HEAP),
            11 => Ok(Self::STACK),
            12 => Ok(Self::ABS),
            13 => Ok(Self::EXT),
            14 => Ok(Self::UNK),
            15 => Ok(Self::NONE),
            _ => Err(()),
        }
    }
}

impl TryFrom<u8> for RefUnknown {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::PLUS),
            1 => Ok(Self::EQ),
            2 => Ok(Self::MINUS),
            _ => Err(()),
        }
    }
}

impl TryFrom<u8> for RefType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::IMM),
            2 => Ok(Self::IMM2),
            3 => Ok(Self::WORD),
            4 => Ok(Self::JUMP),
            5 => Ok(Self::IMM3),
            _ => Err(()),
        }
    }
}
