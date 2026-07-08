pub const LUMIE_MAGIC_LKRN: u32 = 0x4E4D554C;
pub const LUMIE_MAGIC_LDRV: u32 = 0x5652444C;
pub const LUMIE_MAGIC_LSH: u32 = 0x48534C4C;
pub const LUMIE_HDR_SIZE: u32 = 64;

pub const LDRV_CORE: u32 = 1;
pub const LDRV_DISPLAY: u32 = 2;
pub const LDRV_INPUT: u32 = 3;
pub const LDRV_FILESYSTEM: u32 = 4;
pub const LDRV_NETWORK: u32 = 5;
pub const LDRV_EDITOR: u32 = 6;
pub const LSH_SHELL: u32 = 1;

pub const PE_DOS_MAGIC: u16 = 0x5A4D;
pub const PE_NT_SIGNATURE: u32 = 0x00004550;
pub const PE_MACHINE_I386: u16 = 0x014C;
pub const PE_MACHINE_AMD64: u16 = 0x8664;
pub const PE_MACHINE_ARM64: u16 = 0xAA64;
pub const PE_FILE_EXECUTABLE: u16 = 0x0002;
pub const PE_FILE_DLL: u16 = 0x2000;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LumieModHeader {
    pub magic: u32,
    pub ver_major: u16,
    pub ver_minor: u16,
    pub hdr_size: u32,
    pub data_size: u32,
    pub data_off: u32,
    pub checksum: u32,
    pub subtype: u32,
    pub name: [u8; 36],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PeDosHeader {
    pub e_magic: u16,
    pub e_cblp: u16,
    pub e_cp: u16,
    pub e_crlc: u16,
    pub e_cparhdr: u16,
    pub e_minalloc: u16,
    pub e_maxalloc: u16,
    pub e_ss: u16,
    pub e_sp: u16,
    pub e_csum: u16,
    pub e_ip: u16,
    pub e_cs: u16,
    pub e_lfarlc: u16,
    pub e_ovno: u16,
    pub e_res: [u16; 4],
    pub e_oemid: u16,
    pub e_oeminfo: u16,
    pub e_res2: [u16; 10],
    pub e_lfanew: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PeNtHeaders {
    pub signature: u32,
    pub machine: u16,
    pub n_sections: u16,
    pub timedatestamp: u32,
    pub p_symboltable: u32,
    pub n_symbols: u32,
    pub opthdr_size: u16,
    pub characteristics: u16,
}

pub unsafe fn pe_check(buf: *const u8, size: u32) -> i32 {
    if buf.is_null() || size < (core::mem::size_of::<PeDosHeader>() + 4) as u32 {
        return 0;
    }
    let dos = &*(buf as *const PeDosHeader);
    if dos.e_magic != PE_DOS_MAGIC {
        return 0;
    }
    let pe_off = dos.e_lfanew;
    if pe_off + core::mem::size_of::<PeNtHeaders>() as u32 > size {
        return 0;
    }
    let nt = &*(buf.add(pe_off as usize) as *const PeNtHeaders);
    if nt.signature == PE_NT_SIGNATURE { 1 } else { 0 }
}

pub unsafe fn pe_type(buf: *const u8, size: u32) -> *const u8 {
    if pe_check(buf, size) == 0 {
        return core::ptr::null();
    }
    let dos = &*(buf as *const PeDosHeader);
    let nt = &*(buf.add(dos.e_lfanew as usize) as *const PeNtHeaders);
    if nt.characteristics & PE_FILE_DLL != 0 {
        b"DLL\0" as *const u8
    } else {
        b"EXE\0" as *const u8
    }
}

pub unsafe fn pe_machine_str(buf: *const u8, size: u32) -> *const u8 {
    if pe_check(buf, size) == 0 {
        return core::ptr::null();
    }
    let dos = &*(buf as *const PeDosHeader);
    let nt = &*(buf.add(dos.e_lfanew as usize) as *const PeNtHeaders);
    match nt.machine {
        PE_MACHINE_I386 => b"i386\0" as *const u8,
        PE_MACHINE_AMD64 => b"x86_64\0" as *const u8,
        PE_MACHINE_ARM64 => b"ARM64\0" as *const u8,
        _ => b"unknown\0" as *const u8,
    }
}

pub unsafe fn lumie_xor32(data: *const u8, len: u32) -> u32 {
    let mut x: u32 = 0;
    for i in 0..len as usize {
        x ^= (*data.add(i) as u32) << ((i as u32 & 3) * 8);
    }
    x
}

pub unsafe fn lumie_pack_module(
    _data: *const u8,
    data_sz: u32,
    _magic: u32,
    _subtype: u32,
    _name: *const u8,
    _out: *mut *mut u8,
    _out_sz: *mut u32,
) -> i32 {
    let _ = data_sz;
    -1
}

pub unsafe fn lumie_pack_module_kmalloc(
    data: *const u8,
    data_sz: u32,
    magic: u32,
    subtype: u32,
    name: *const u8,
    out: *mut *mut u8,
    out_sz: *mut u32,
) -> i32 {
    lumie_pack_module(data, data_sz, magic, subtype, name, out, out_sz)
}
