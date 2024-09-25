
# Relocation Entry

```
typedef struct RelocationEntry {
    uint32_t address;
    uint8_t section;
    uint8_t _pad[2];
    uint8_t info;
}
```

0----+----+----+----4----+----+----+----8
|    address (BE)   |sect|  Info (BE)   |
+----+----+----+----+----+----+----+----+

sect is 1 byte wide and is the following enum

```
typedef enum REL_SECT {
    TEXT = 0;
    RDATA = 1;
    DATA = 2;
    SDATA = 3;
    SBSS = 4;
    BSS = 5;
};
```

Info is 3 bytes wide and only the lowest order byte (index 7) contains
relevant information

```
typedef enum REL_INFO {
    IMM = 1;
    IMM2 = 2;
    WORD = 3;
    JUMP = 4;
    IMM3 = 5;
}
```

# Reference Entry

```
typedef struct ReferenceEntry {
    uint32_t address;
    uint32_t strtab_offset;
    uint32_t ref_info;
}
```

0----+----+----+----4----+----+----+----8----+----+----+----b
|    address (BE)   | strtab offset (BE)| reference info    |
+----+----+----+----+----+----+----+----+----+----+----+----+

reference info has the following bitfields

0----+----4----+----8----+----b----+---10----+---14----+---18----+---1b----+---20
|               ix (BE)                 |res | ?? |   typ   |       sect        |
+----+----+----+----+----+----+----+----+----+----+----+----+----+----+----+----+

res is reserved / ignored

?? is some kind of enum, with the following values

typedef enum UNKNOWN {
    PLUS = 0;
    EQ = 1;
    MINUS = 2;
}

typ is the type of data being referenced, and shares values with REL_INFO

typedef enum REF_TYP {
    IMM = 1;
    IMM2 = 2;
    WORD = 3;
    JUMP = 4;
    IMM3 = 5;
}

sect is the section where the data being referenced exists, and shares values with REL_SECT

typedef enum REF_SECT {
    TEXT = 0;
    RDATA = 1;
    DATA = 2;
    SDATA = 3;
    SBSS = 4;
    BSS = 5;
}

# Symbol Entry

typedef struct SymtabEntry {
    uint32_t flags;
    uint32_t value;
    uint32_t strtab_offset;
    uint16_t ofid;
    uint16_t _pad;
}

0----+----+----+----4----+----+----+----8----+----+----+----b----+----+----+---10
|       flags       |     value (BE)    |strtab offset (BE) |ofid (BE)|  pad    |
+----+----+----+----+----+----+----+----+----+----+----+----+----+----+----+----+

the lowest order 4 bits of `flags` determines in which section the symbol is located

typedef enum SYM_LOC {
    S_TEXT = 0;   // Text (code)
    S_RDATA = 1;  // Read only data
    S_DATA = 2;   // Data
    S_SDATA = 3;  // "small" Data
    S_SBSS = 4;   // Small unitialized data
    S_BSS = 5;    // Uninitialized data
    S_REL = 6;    // relocation table
    S_REF = 7;    // reference table
    S_SYM = 8;    // symbol table
    S_STR = 9;    // string table
    S_HEAP = 10;  // heap
    S_STACK = 11; // stack
    S_ABS = 12;   // absolute expression?
    S_EXT = 13;   // external?
    S_UNK = 14;   // unknown
    S_NONE = 15;  // none
}

Other flags are as follows

#define FORW    0x0000_0010  // Forward?
#define RELOC   0x0000_0020  // Relocatable; Implies defined
#define EQ      0x0000_0040  // Equate / const assign
#define LBL     0x0000_0080  // Label
#define REG     0x0000_0100  // Register
#define PRE     0x0000_0200  // ??
#define UNDEF   0x0000_0400  // Symbol not defined
#define XTV     0x0000_0800  // ??
#define MUL     0x0000_1000  // ??
#define RPT     0x0000_2000  // Repeat expression?
#define GLB     0x0000_4000  // Global; Implies EXTERN
#define SML     0x0000_8000  // Small?
#define ADJ     0x0001_0000  // ??
#define DISC    0x0002_0000  // ??
#define LIT     0x0004_0000  // Implies defined

All symbols which are not RELOC or LIT are UNDEF

Hex 0x0008_0000 through 0x8000_0000 are reserved

