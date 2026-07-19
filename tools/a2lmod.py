#!/usr/bin/env python3
"""Convert a `.a` (AR archive of COFF objects) to Lumie module format.

Usage:
    python a2lmod.py <input.a> <output.lkrn|lsh|ldrv> --magic LKRN|LSH|LDRV --name "ModuleName"

The tool uses lld-link (from Rust toolchain) to link the archive into a PE DLL,
then converts the PE to Lumie module format (LshHeader + code + relocations).
"""

import struct
import sys
import os
import subprocess
import tempfile
import shutil

# Lumie module magic constants
MAGIC_LKRN = 0x4E524B4C  # "LKRN"
MAGIC_LSH  = 0x48534C4C  # "LLSH"
MAGIC_LDRV = 0x5652444C  # "LDRV"
MAGIC_SYS  = 0x01535953  # "SYS\x01"

# PE constants
IMAGE_DOS_SIGNATURE   = 0x5A4D  # "MZ"
IMAGE_NT_SIGNATURE    = 0x00004550  # "PE\0\0"
IMAGE_NT_OPTIONAL_HDR32_MAGIC = 0x10B
IMAGE_NT_OPTIONAL_HDR64_MAGIC = 0x20B
IMAGE_SIZEOF_SHORT_NAME = 8

# COFF machine types
IMAGE_FILE_MACHINE_AMD64 = 0x8664

# Section characteristics
IMAGE_SCN_CNT_CODE      = 0x00000020
IMAGE_SCN_CNT_INITIALIZED_DATA = 0x00000040
IMAGE_SCN_MEM_DISCARDABLE = 0x02000000

# Relocation types
IMAGE_REL_BASED_ABSOLUTE = 0
IMAGE_REL_BASED_DIR64    = 10

PE_DIR_BASERELOC = 5  # Data directory index for base relocations

def find_lld_link():
    """Find lld-link in Rust toolchain or system."""
    import glob
    # Check environment variable first
    env_lld = os.environ.get("LLD_LINK")
    if env_lld and os.path.exists(env_lld):
        return env_lld
    # Linux: try rust-lld from nightly toolchain
    for toolchain_dir in glob.glob(os.path.expanduser("~/.rustup/toolchains/nightly-*/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld")):
        if os.path.exists(toolchain_dir):
            return toolchain_dir
    # Linux: try /tmp/lld-link symlink
    if os.path.exists("/tmp/lld-link"):
        return "/tmp/lld-link"
    # Windows: try the known path
    known = os.path.expanduser(
        "~/.rustup/toolchains/1.89.0-x86_64-pc-windows-msvc/"
        "lib/rustlib/x86_64-pc-windows-msvc/bin/gcc-ld/lld-link.exe"
    )
    if os.path.exists(known):
        return known
    # Windows: search for lld-link.exe
    for root, dirs, files in os.walk(os.path.expanduser("~/.rustup")):
        for f in files:
            if f.lower() == "lld-link.exe":
                return os.path.join(root, f)
    return None


def link_to_pe_dll(input_a, output_dll):
    """Link a .a file into a PE DLL using lld-link."""
    lld = find_lld_link()
    if not lld:
        print("ERROR: lld-link not found in Rust toolchain.", file=sys.stderr)
        sys.exit(1)
    
    # Normalize path for lld-link (use backslashes on Windows)
    abs_input = os.path.abspath(input_a)
    
    cmd = [
        lld, "/dll", "/noentry", "/nodefaultlib",
        "/machine:x64", "/nologo",
        "/lldmingw",  # Handle GNU ar format archives
        "/base:0",
        f"/out:{output_dll}",
        f"/wholearchive:{abs_input}",  # Force include ALL objects
    ]
    
    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"lld-link failed (rc={result.returncode}): {result.stderr}", file=sys.stderr)
        if result.stdout:
            print(f"stdout: {result.stdout}", file=sys.stderr)
        sys.exit(1)
    if result.stderr:
        print(f"lld-link warnings: {result.stderr}", file=sys.stderr)


def align_up(val, align):
    return (val + align - 1) & ~(align - 1)


def read_pe_sections(data, pe_offset, num_sections, opt_hdr_size):
    """Read PE section table."""
    sections = []
    offset = pe_offset + 24 + opt_hdr_size  # Skip COFF header + optional header
    
    sects = data[offset:offset + num_sections * 40]
    for i in range(num_sections):
        sec = sects[i*40:(i+1)*40]
        name = sec[:8].rstrip(b'\x00').decode('ascii', errors='replace')
        virtual_size    = struct.unpack_from('<I', sec, 8)[0]
        virtual_address = struct.unpack_from('<I', sec, 12)[0]
        size_of_raw_data = struct.unpack_from('<I', sec, 16)[0]
        ptr_raw_data    = struct.unpack_from('<I', sec, 20)[0]
        characteristics = struct.unpack_from('<I', sec, 36)[0]
        
        sections.append({
            'name': name,
            'virtual_size': virtual_size,
            'virtual_address': virtual_address,
            'raw_size': size_of_raw_data,
            'raw_offset': ptr_raw_data,
            'characteristics': characteristics,
        })
    
    return sections


def extract_section_data(data, section):
    """Extract section data from the PE file."""
    raw_off = section['raw_offset']
    raw_sz = section['raw_size']
    virt_sz = section['virtual_size']
    
    if raw_off == 0 or raw_sz == 0:
        return b''
    
    actual_size = min(raw_sz, virt_sz)
    result = data[raw_off:raw_off + actual_size]
    
    # Pad if raw is smaller than virtual (uninitialized data)
    if actual_size < virt_sz:
        result += b'\x00' * (virt_sz - actual_size)
    
    return result


def parse_base_relocs(data, sections):
    """Parse PE .reloc section and return list of (RVA, type) tuples."""
    # Find the .reloc section
    reloc_sec = None
    for sec in sections:
        if sec['name'] == '.reloc':
            reloc_sec = sec
            break
    
    if not reloc_sec:
        return []
    
    raw_off = reloc_sec['raw_offset']
    raw_sz = reloc_sec['raw_size']
    
    if raw_off == 0 or raw_sz == 0:
        return []
    
    reloc_data = data[raw_off:raw_off + raw_sz]
    relocs = []
    
    pos = 0
    while pos + 8 <= len(reloc_data):
        page_rva = struct.unpack_from('<I', reloc_data, pos)[0]
        block_size = struct.unpack_from('<I', reloc_data, pos + 4)[0]
        
        if block_size < 8:
            break
        
        num_entries = (block_size - 8) // 2
        entries = reloc_data[pos + 8:pos + block_size]
        
        for i in range(num_entries):
            entry = struct.unpack_from('<H', entries, i * 2)[0]
            reloc_type = entry >> 12
            reloc_offset = entry & 0xFFF
            target_rva = page_rva + reloc_offset
            
            if reloc_type != IMAGE_REL_BASED_ABSOLUTE:
                relocs.append((target_rva, reloc_type))
        
        pos += block_size
    
    return relocs


def convert_pe_to_lumie(data, magic, name, output_path):
    """Convert PE DLL to Lumie module format."""
    # Parse DOS header
    dos_magic = struct.unpack_from('<H', data, 0)[0]
    assert dos_magic == IMAGE_DOS_SIGNATURE, "Not a valid PE file"
    
    e_lfanew = struct.unpack_from('<I', data, 0x3C)[0]
    
    # Parse NT headers
    nt_sig = struct.unpack_from('<I', data, e_lfanew)[0]
    assert nt_sig == IMAGE_NT_SIGNATURE, "Not a valid PE file"
    
    coff = e_lfanew + 4
    machine     = struct.unpack_from('<H', data, coff)[0]
    num_sections = struct.unpack_from('<H', data, coff + 2)[0]
    opt_hdr_size = struct.unpack_from('<H', data, coff + 16)[0]
    
    assert machine == IMAGE_FILE_MACHINE_AMD64, "Only x86-64 supported"
    
    opt = coff + 20
    magic_opt = struct.unpack_from('<H', data, opt)[0]
    assert magic_opt == IMAGE_NT_OPTIONAL_HDR64_MAGIC, "Only PE32+ supported"
    
    image_base    = struct.unpack_from('<Q', data, opt + 24)[0]
    entry_rva     = struct.unpack_from('<I', data, opt + 16)[0]
    section_align = struct.unpack_from('<I', data, opt + 32)[0]
    
    # Read base relocations from data directory
    reloc_dir_rva  = struct.unpack_from('<I', data, opt + 96 + PE_DIR_BASERELOC * 8)[0]
    reloc_dir_size = struct.unpack_from('<I', data, opt + 96 + PE_DIR_BASERELOC * 8 + 4)[0]
    
    # Read sections
    sections = read_pe_sections(data, e_lfanew, num_sections, opt_hdr_size)
    
    # Find first section RVA (base for offset calculation)
    if not sections:
        print("ERROR: No sections in PE file", file=sys.stderr)
        sys.exit(1)
    
    first_rva = min(s['virtual_address'] for s in sections)
    
    # Collect initialized sections that are not discardable
    # Sort by RVA
    init_sections = sorted(
        [s for s in sections
         if (s['characteristics'] & (IMAGE_SCN_CNT_CODE | IMAGE_SCN_CNT_INITIALIZED_DATA))
         and not (s['characteristics'] & IMAGE_SCN_MEM_DISCARDABLE)],
        key=lambda s: s['virtual_address']
    )
    
    # Build flat code blob with gaps
    code_blob = bytearray()
    last_end = first_rva
    
    section_map = {}  # RVA -> offset in flat blob
    for sec in init_sections:
        gap = sec['virtual_address'] - last_end
        if gap > 0:
            code_blob += b'\x00' * gap
        
        section_map[sec['virtual_address']] = len(code_blob)
        
        sec_data = extract_section_data(data, sec)
        code_blob += sec_data
        
        last_end = sec['virtual_address'] + sec['virtual_size']
    
    code_size = len(code_blob)
    
    # Convert base relocations
    # First try data directory, then fall back to section table
    all_relocs = parse_base_relocs(data, sections)
    
    # Filter relocs to only those within initialized sections and convert offsets
    lumie_relocs = []
    for rva, rtype in all_relocs:
        if rtype != IMAGE_REL_BASED_DIR64:
            continue
        
        # Find which section this RVA falls in and compute flat blob offset
        for sec in init_sections:
            sec_start = sec['virtual_address']
            sec_end = sec_start + sec['virtual_size']
            if sec_start <= rva < sec_end:
                # Flat offset = offset of section in blob + (RVA - section_start)
                flat_off = section_map[sec_start] + (rva - sec_start)
                lumie_relocs.append(flat_off)
                
                # Also need to adjust the value: subtract image_base
                # The value at this RVA is currently: image_base + X
                # We want it to be: X (offset within image)
                # So subtract image_base from the value
                value_off = sec['raw_offset'] + (rva - sec_start)
                if value_off + 8 <= len(data):
                    val = struct.unpack_from('<Q', data, value_off)[0]
                    adjusted = val - image_base
                    struct.pack_into('<Q', code_blob, flat_off, adjusted)
                break
    
    # Convert PE entry point RVA to flat blob offset
    entry_flat = 0
    if entry_rva != 0:
        for sec in init_sections:
            sec_start = sec['virtual_address']
            sec_end = sec_start + sec['virtual_size']
            if sec_start <= entry_rva < sec_end:
                entry_flat = section_map[sec_start] + (entry_rva - sec_start)
                break

    # Build relocation table (array of u32)
    reloc_data = b''
    for off in lumie_relocs:
        reloc_data += struct.pack('<I', off)
    
    reloc_count = len(lumie_relocs)
    reloc_off = 0  # Will be set after header
    
    # Build LshHeader
    name_bytes = name.encode('ascii', errors='replace')[:23]
    name_bytes = name_bytes.ljust(24, b'\x00')  # Pad to 24 bytes
    
    hdr = struct.pack('<IIIIIIIIII',  # 10 fields of 4 bytes each (40 bytes)
        magic,          # magic
        entry_flat,     # entry (flat offset into code blob)
        code_size,      # code_size
        0,              # bss_size
        0,              # reloc_off (placeholder)
        reloc_count,    # reloc_count
        0,              # import_off
        0,              # import_count
        0,              # strtab_off
        0,              # strtab_size
    )
    hdr += name_bytes  # 24 bytes -> total 64 bytes
    
    # Set reloc_off: right after header + code
    reloc_off = 64 + code_size
    hdr = hdr[:16] + struct.pack('<I', reloc_off) + hdr[20:]
    
    # Write output
    with open(output_path, 'wb') as f:
        f.write(hdr)
        f.write(bytes(code_blob))
        f.write(reloc_data)
    
    print(f"Created {output_path}:")
    print(f"  Magic:      0x{magic:08X} ({magic_str(magic)})")
    print(f"  Entry:      0x{entry_flat:X}")
    print(f"  Code size:  0x{code_size:X} ({code_size} bytes)")
    print(f"  Relocs:     {reloc_count}")
    print(f"  Total:      {64 + code_size + len(reloc_data)} bytes")


def magic_str(magic):
    m = struct.pack('<I', magic)
    return m.decode('ascii', errors='replace')


def main():
    if len(sys.argv) < 4:
        print(__doc__, file=sys.stderr)
        print("\nOptions:", file=sys.stderr)
        print("  --magic LKRN|LSH|LDRV|SYS   Module type (default: LKRN)", file=sys.stderr)
        print("  --name \"Name\"               Module name (default: from filename)", file=sys.stderr)
        sys.exit(1)
    
    input_a = sys.argv[1]
    output_path = sys.argv[2]
    
    # Parse options
    magic_type = "LKRN"
    name = os.path.splitext(os.path.basename(output_path))[0]
    
    i = 3
    while i < len(sys.argv):
        if sys.argv[i] == "--magic" and i + 1 < len(sys.argv):
            magic_type = sys.argv[i + 1].upper()
            i += 2
        elif sys.argv[i] == "--name" and i + 1 < len(sys.argv):
            name = sys.argv[i + 1]
            i += 2
        else:
            print(f"Unknown option: {sys.argv[i]}", file=sys.stderr)
            sys.exit(1)
    
    magic_map = {
        "LKRN": MAGIC_LKRN,
        "LSH": MAGIC_LSH,
        "LDRV": MAGIC_LDRV,
        "SYS": MAGIC_SYS,
    }
    
    if magic_type not in magic_map:
        print(f"Unknown magic type: {magic_type}. Use LKRN, LSH, LDRV, or SYS.", file=sys.stderr)
        sys.exit(1)
    
    magic = magic_map[magic_type]
    
    if not os.path.exists(input_a):
        print(f"Input file not found: {input_a}", file=sys.stderr)
        sys.exit(1)
    
    # Create temp directory for linking
    tmpdir = tempfile.mkdtemp(prefix="a2lmod_")
    try:
        dll_path = os.path.join(tmpdir, "temp_module.dll")
        
        print(f"Linking {input_a} -> {dll_path} ...")
        link_to_pe_dll(input_a, dll_path)
        
        if not os.path.exists(dll_path):
            print(f"ERROR: DLL not created at {dll_path}", file=sys.stderr)
            sys.exit(1)
        
        dll_size = os.path.getsize(dll_path)
        print(f"DLL created: {dll_size} bytes")
        
        print(f"Reading {dll_path} ...")
        with open(dll_path, 'rb') as f:
            pe_data = f.read()
        
        os.makedirs(os.path.dirname(output_path) or '.', exist_ok=True)
        convert_pe_to_lumie(pe_data, magic, name, output_path)
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)


if __name__ == '__main__':
    main()
