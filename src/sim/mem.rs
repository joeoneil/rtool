use super::{DATA_START, PAGE_BITS, PAGE_MASK, PAGE_SIZE, STACK_SIZE, STACK_START, TEXT_START};
use crate::common::Error;
use crate::sim::ObjectModule;

use std::collections::HashMap;

/// struct the manages the virtual address space the running program is in.
/// Controls reads and writes to and from memory as well as allocating pages
#[derive(Clone)]
pub struct Memory {
    /// Page table mapping virtual ids (20 most significant bits of ptr) to
    /// real ids.
    pub table: HashMap<PageID, PageID>,
    /// flag mapping virtual ids to the writability of a given page.
    pub write: HashMap<PageID, bool>,
    /// flag mapping virtual ids to the executability of a given page.
    pub exec: HashMap<PageID, bool>,
    /// buffer containing all pages.
    pub pages: Vec<Page>,
}

/// Thin wrapper around u32. Will not be greater than 20 bits long. Larger IDs
/// are inaccessible
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageID(pub u32);

#[derive(Clone, Copy)]
pub struct Page(pub [u8; PAGE_SIZE as usize]);

impl Memory {
    #[inline]
    fn map_virt_to_real(&self, addr: u32) -> Option<u32> {
        let virt_page = PageID(addr >> PAGE_BITS);
        let page_addr = addr & PAGE_MASK;
        let real_page = self.table.get(&virt_page)?;
        Some((real_page.0 << PAGE_BITS) | page_addr)
    }

    pub fn read_word(&self, addr: u32) -> Result<u32, Error> {
        if addr % 4 != 0 {
            Err(Error::MemoryAccessError(format!(
                "Unaligned memory access at 0x{:08x}",
                addr,
            )))
        } else if let Some(addr) = self.map_virt_to_real(addr) {
            let page_id = addr >> PAGE_BITS;
            let page_addr = (addr & PAGE_MASK);
            Ok(u32::from_be_bytes(
                self.pages
                    .get(page_id as usize)
                    .expect("Unmapped page in page table")
                    .0[page_addr as usize..page_addr as usize + 4]
                    .try_into()
                    .unwrap(),
            ))
        } else {
            Err(Error::MemoryAccessError(format!(
                "Attempted to access unmapped page with read at 0x{:08x} (PageID {})",
                addr,
                (addr >> PAGE_BITS),
            )))
        }
    }

    pub fn read_half(&self, addr: u32) -> Result<u16, Error> {
        if addr % 2 != 0 {
            Err(Error::MemoryAccessError(format!(
                "Unaligned memory access at 0x{:08x}",
                addr,
            )))
        } else if let Some(addr) = self.map_virt_to_real(addr) {
            let page_id = addr >> PAGE_BITS;
            let page_addr = (addr & PAGE_MASK);
            Ok(u16::from_be_bytes(
                self.pages
                    .get(page_id as usize)
                    .expect("Unmapped page in page table")
                    .0[page_addr as usize..page_addr as usize + 2]
                    .try_into()
                    .unwrap(),
            ))
        } else {
            Err(Error::MemoryAccessError(format!(
                "Attempted to access unmapped page with read at 0x{:08x}",
                addr
            )))
        }
    }

    pub fn read_byte(&self, addr: u32) -> Result<u8, Error> {
        if let Some(addr) = self.map_virt_to_real(addr) {
            let page_id = addr >> PAGE_BITS;
            let page_addr = (addr & PAGE_MASK);
            Ok(self
                .pages
                .get(page_id as usize)
                .expect("PANIC: Unmapped page in page table")
                .0[page_addr as usize])
        } else {
            Err(Error::MemoryAccessError(format!(
                "Attempted to access unmapped page with read at 0x{:08x}",
                addr
            )))
        }
    }

    pub fn write_word(&mut self, addr: u32, value: u32) -> Result<(), Error> {
        if addr % 4 != 0 {
            Err(Error::MemoryAccessError(format!(
                "Unaligned memory access @ {:08x}",
                addr,
            )))
        } else if let Some(real_addr) = self.map_virt_to_real(addr) {
            let page_id = real_addr >> PAGE_BITS;
            let mut page_addr = (real_addr & PAGE_MASK);
            if !self
                .write
                .get(&PageID(addr >> PAGE_BITS))
                .expect("Unmapped page in page table")
            {
                Err(Error::MemoryAccessError(format!(
                    "Attempted to write to read-only page @ 0x{:08x}",
                    addr
                )))
            } else {
                let buf = value.to_be_bytes();
                let p = self
                    .pages
                    .get_mut(page_id as usize)
                    .expect("Unmapped page in page table");
                for b in buf {
                    p.0[page_addr as usize] = b;
                    page_addr += 1;
                }
                Ok(())
            }
        } else {
            Err(Error::MemoryAccessError(format!(
                "Attempted to access unmapped page with read @ 0x{:08x}",
                addr
            )))
        }
    }

    pub fn write_half(&mut self, addr: u32, value: u16) -> Result<(), Error> {
        if addr % 2 != 0 {
            Err(Error::MemoryAccessError(format!(
                "Unaligned memory access at 0x{:08x}",
                addr,
            )))
        } else if let Some(real_addr) = self.map_virt_to_real(addr) {
            let page_id = real_addr >> PAGE_BITS;
            let mut page_addr = (real_addr & PAGE_MASK);
            if !self
                .write
                .get(&PageID(addr >> PAGE_BITS))
                .expect("Unmapped page in page table")
            {
                Err(Error::MemoryAccessError(format!(
                    "Attempted to write to read-only page @ 0x{:08x}",
                    addr
                )))
            } else {
                let buf = value.to_be_bytes();
                let p = self
                    .pages
                    .get_mut(page_id as usize)
                    .expect("Unmapped page in page table");
                for b in buf {
                    p.0[page_addr as usize] = b;
                    page_addr += 1;
                }
                Ok(())
            }
        } else {
            Err(Error::MemoryAccessError(format!(
                "Attempted to access unmapped page with read @ 0x{:08x}",
                addr
            )))
        }
    }

    pub fn write_byte(&mut self, addr: u32, value: u8) -> Result<(), Error> {
        if let Some(real_addr) = self.map_virt_to_real(addr) {
            let page_id = real_addr >> PAGE_BITS;
            let page_addr = (real_addr & PAGE_MASK);
            if !self
                .write
                .get(&PageID(addr >> PAGE_BITS))
                .expect("PANIC: Unmapped page in page table")
            {
                Err(Error::MemoryAccessError(format!(
                    "Attempted to write to read-only page @ 0x{:08x}",
                    addr
                )))
            } else {
                let l = self.pages.len();
                self.pages
                    .get_mut(page_id as usize)
                    .expect("PANIC: Unmapped page in page table")
                    .0[page_addr as usize] = value;
                Ok(())
            }
        } else {
            Err(Error::MemoryAccessError(format!(
                "Attempted to access unmapped page with write @ 0x{:08x}",
                addr
            )))
        }
    }

    pub fn check_exec(&self, addr: u32) -> Option<bool> {
        let addr = self.map_virt_to_real(addr)?;
        let page_id = (addr << PAGE_BITS);
        self.exec.get(&PageID(page_id)).copied()
    }

    /// Will panic if the allocated page real_id exceeds (1 << 20), meaning
    /// the program cannot allocate more than 4GB of memory.
    pub fn alloc_page(&mut self, v_addr: u32, write: bool, exec: bool) -> Option<&mut Page> {
        let real_id = PageID(self.pages.len() as u32);
        let virt_id = PageID(v_addr >> PAGE_BITS);

        if real_id.0 >= (1 << (32 - PAGE_BITS)) {
            panic!("Out of memory Exception");
        }

        if self.table.contains_key(&virt_id) {
            None
        } else {
            self.table.insert(virt_id, real_id);
            self.pages.push(Page([0u8; PAGE_SIZE as usize]));
            self.write.insert(virt_id, write);
            self.exec.insert(virt_id, exec);
            self.pages.get_mut(real_id.0 as usize)
        }
    }

    #[inline]
    pub fn get_raw_page_virt(&mut self, v_id: PageID) -> Option<&mut Page> {
        let real_id = self.table.get(&v_id)?;
        self.pages.get_mut(real_id.0 as usize)
    }

    #[inline]
    pub fn get_raw_page_real(&mut self, r_id: PageID) -> Option<&mut Page> {
        self.pages.get_mut(r_id.0 as usize)
    }

    pub(super) fn new() -> Self {
        Self {
            table: HashMap::new(),
            write: HashMap::new(),
            exec: HashMap::new(),
            pages: Vec::new(),
        }
    }

    pub fn alloc_data(&mut self, mut base_addr: u32, data: &[u8], write: bool, exec: bool) -> u32 {
        let mut iter = data.iter().copied().peekable();
        while iter.peek().is_some() {
            let mut p = self.alloc_page(base_addr, write, exec).unwrap();
            base_addr += PAGE_SIZE;
            let data = iter.by_ref().take(PAGE_SIZE as usize).collect::<Vec<_>>();
            for (idx, b) in data.into_iter().enumerate() {
                p.0[idx] = b;
            }
        }

        base_addr
    }

    pub fn new_from_object(module: ObjectModule) -> Self {
        let mut s = Self::new();

        // Create program memory image
        s.alloc_data(TEXT_START, module.text.as_slice(), false, true);
        let data_start = s.alloc_data(DATA_START, module.rdata.as_slice(), false, false);
        let sdata_start = s.alloc_data(data_start, module.data.as_slice(), true, false);
        let sbss_start = s.alloc_data(sdata_start, module.sdata.as_slice(), true, false);
        let bss_start = s.alloc_data(
            sbss_start,
            [0].into_iter()
                .cycle()
                .take(module.head.data[4] as usize)
                .collect::<Vec<_>>()
                .as_slice(),
            true,
            false,
        );
        let heap_start = s.alloc_data(
            bss_start,
            [0].into_iter()
                .cycle()
                .take(module.head.data[5] as usize)
                .collect::<Vec<_>>()
                .as_slice(),
            true,
            false,
        );

        // Alloc stack
        let mut stack_remaining = STACK_SIZE;
        let mut next_stack_addr = STACK_START;
        while stack_remaining > 0 {
            s.alloc_page(next_stack_addr, true, false);
            next_stack_addr -= PAGE_SIZE;
            stack_remaining -= PAGE_SIZE;
        }

        s
    }

    pub fn dump_page_table(&self, print_stack: bool) -> () {
        let mut kv = self.table.iter().collect::<Vec<_>>();
        kv.sort_by_key(|k| k.0);
        println!("Page table has {} pages alloc'd", self.pages.len());
        println!(
            "Page table dump: {}",
            if !print_stack {
                "(Excluding stack memory)"
            } else {
                ""
            }
        );
        for (k, v) in kv {
            if !print_stack && (k.0 << PAGE_BITS) >= (STACK_START - STACK_SIZE) {
                continue;
            }
            println!(
                "0x{:08x} [{}] -> 0x{:08x} [{}]",
                (k.0 << PAGE_BITS),
                k.0,
                (v.0 << PAGE_BITS),
                v.0
            );
        }
    }
}
