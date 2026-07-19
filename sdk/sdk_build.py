#!/usr/bin/env python3
"""
LumieOS SDK Build Tool
=====================

Unified build script for LumieOS modules (apps, drivers, kernel extensions).

Usage:
    python sdk_build.py <command> [options]

Commands:
    app <name>       Build a .lsh application module
    driver <name>    Build a .ldrv driver module  
    kernel           Build the kernel module
    lumiec <file>    Compile a LumieC source file to .sys
    clean            Clean build artifacts
    package          Create install package from all built modules

Examples:
    python sdk_build.py app myapp --release
    python sdk_build.py driver mydriver
    python sdk_build.py lumiec hello.lc -o hello.sys
    python sdk_build.py package
"""

import sys
import os
import subprocess
import shutil
import tempfile
import struct
import glob

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
BUILD_DIR = os.path.join(PROJECT_ROOT, "build")
SDK_DIR = SCRIPT_DIR

MAGIC_LKRN = 0x4E524B4C
MAGIC_LSH  = 0x48534C4C
MAGIC_LDRV = 0x5652444C
MAGIC_SYS  = 0x01535953

CARGO_TARGET = "x86_64-unknown-uefi"


def find_cargo():
    cargo = shutil.which("cargo")
    if cargo:
        return cargo
    for root, dirs, files in os.walk(os.path.expanduser("~/.cargo/bin")):
        for f in files:
            if f.startswith("cargo"):
                return os.path.join(root, f)
    return "cargo"


def find_lld_link():
    import glob as g
    env_lld = os.environ.get("LLD_LINK")
    if env_lld and os.path.exists(env_lld):
        return env_lld
    for toolchain_dir in g.glob(os.path.expanduser(
        "~/.rustup/toolchains/nightly-*/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld"
    )):
        if os.path.exists(toolchain_dir):
            return toolchain_dir
    if os.path.exists("/tmp/lld-link"):
        return "/tmp/lld-link"
    known = os.path.expanduser(
        "~/.rustup/toolchains/1.89.0-x86_64-pc-windows-msvc/"
        "lib/rustlib/x86_64-pc-windows-msvc/bin/gcc-ld/lld-link.exe"
    )
    if os.path.exists(known):
        return known
    for root, dirs, files in os.walk(os.path.expanduser("~/.rustup")):
        for f in files:
            if f.lower() == "lld-link.exe":
                return os.path.join(root, f)
    return None


def cargo_build(crate_path, release=True):
    cargo = find_cargo()
    cmd = [cargo, "build"]
    if release:
        cmd.append("--release")
    cmd.extend(["--target", CARGO_TARGET, "--manifest-path", os.path.join(crate_path, "Cargo.toml")])
    print(f"[SDK] Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=PROJECT_ROOT)
    if result.returncode != 0:
        print(f"[SDK] Build failed for {crate_path}", file=sys.stderr)
        sys.exit(1)
    return os.path.join(
        PROJECT_ROOT, "target", CARGO_TARGET,
        "release" if release else "debug"
    )


def find_a_file(crate_name):
    target_dir = os.path.join(PROJECT_ROOT, "target", CARGO_TARGET, "release")
    a_file = os.path.join(target_dir, f"lib{crate_name}.a")
    if os.path.exists(a_file):
        return a_file
    a_file = os.path.join(target_dir, f"{crate_name}.a")
    if os.path.exists(a_file):
        return a_file
    for f in os.listdir(target_dir):
        if f.endswith(".a"):
            return os.path.join(target_dir, f)
    return None


def link_to_pe_dll(input_a, output_dll):
    lld = find_lld_link()
    if not lld:
        print("[SDK] ERROR: lld-link not found", file=sys.stderr)
        sys.exit(1)
    abs_input = os.path.abspath(input_a)
    cmd = [
        lld, "/dll", "/noentry", "/nodefaultlib",
        "/machine:x64", "/nologo",
        "/lldmingw", "/base:0",
        f"/out:{output_dll}",
        f"/wholearchive:{abs_input}",
    ]
    print(f"[SDK] Linking: {lld}")
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"[SDK] lld-link failed: {result.stderr}", file=sys.stderr)
        sys.exit(1)


def align_up(val, align):
    return (val + align - 1) & ~(align - 1)


def read_pe_sections(data, pe_offset, num_sections, opt_hdr_size):
    sections = []
    offset = pe_offset + 24 + opt_hdr_size
    sects = data[offset:offset + num_sections * 40]
    for i in range(num_sections):
        sec = sects[i*40:(i+1)*40]
        name = sec[:8].rstrip(b'\x00').decode('ascii', errors='replace')
        virtual_size = struct.unpack_from('<I', sec, 8)[0]
        virtual_address = struct.unpack_from('<I', sec, 12)[0]
        size_of_raw_data = struct.unpack_from('<I', sec, 16)[0]
        ptr_raw_data = struct.unpack_from('<I', sec, 20)[0]
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
    raw_off = section['raw_offset']
    raw_sz = section['raw_size']
    virt_sz = section['virtual_size']
    if raw_off == 0 or raw_sz == 0:
        return b''
    actual_size = min(raw_sz, virt_sz)
    result = data[raw_off:raw_off + actual_size]
    if actual_size < virt_sz:
        result += b'\x00' * (virt_sz - actual_size)
    return result


def parse_base_relocs(data, sections):
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
            if reloc_type != 0:
                relocs.append((target_rva, reloc_type))
        pos += block_size
    return relocs


def convert_pe_to_lumie(data, magic, name, output_path):
    dos_magic = struct.unpack_from('<H', data, 0)[0]
    assert dos_magic == 0x5A4D, "Not a valid PE file"
    e_lfanew = struct.unpack_from('<I', data, 0x3C)[0]
    nt_sig = struct.unpack_from('<I', data, e_lfanew)[0]
    assert nt_sig == 0x00004550, "Not a valid PE file"
    coff = e_lfanew + 4
    num_sections = struct.unpack_from('<H', data, coff + 2)[0]
    opt_hdr_size = struct.unpack_from('<H', data, coff + 16)[0]
    opt = coff + 20
    image_base = struct.unpack_from('<Q', data, opt + 24)[0]
    entry_rva = struct.unpack_from('<I', data, opt + 16)[0]
    section_align = struct.unpack_from('<I', data, opt + 32)[0]
    sections = read_pe_sections(data, e_lfanew, num_sections, opt_hdr_size)
    if not sections:
        print("[SDK] ERROR: No sections in PE file", file=sys.stderr)
        sys.exit(1)
    first_rva = min(s['virtual_address'] for s in sections)
    init_sections = sorted(
        [s for s in sections
         if (s['characteristics'] & (0x20 | 0x40))
         and not (s['characteristics'] & 0x02000000)],
        key=lambda s: s['virtual_address']
    )
    code_blob = bytearray()
    last_end = first_rva
    section_map = {}
    for sec in init_sections:
        gap = sec['virtual_address'] - last_end
        if gap > 0:
            code_blob += b'\x00' * gap
        section_map[sec['virtual_address']] = len(code_blob)
        sec_data = extract_section_data(data, sec)
        code_blob += sec_data
        last_end = sec['virtual_address'] + sec['virtual_size']
    code_size = len(code_blob)
    all_relocs = parse_base_relocs(data, sections)
    lumie_relocs = []
    for rva, rtype in all_relocs:
        if rtype != 10:
            continue
        for sec in init_sections:
            sec_start = sec['virtual_address']
            sec_end = sec_start + sec['virtual_size']
            if sec_start <= rva < sec_end:
                flat_off = section_map[sec_start] + (rva - sec_start)
                lumie_relocs.append(flat_off)
                value_off = sec['raw_offset'] + (rva - sec_start)
                if value_off + 8 <= len(data):
                    val = struct.unpack_from('<Q', data, value_off)[0]
                    adjusted = val - image_base
                    struct.pack_into('<Q', code_blob, flat_off, adjusted)
                break
    entry_flat = 0
    if entry_rva != 0:
        for sec in init_sections:
            sec_start = sec['virtual_address']
            sec_end = sec_start + sec['virtual_size']
            if sec_start <= entry_rva < sec_end:
                entry_flat = section_map[sec_start] + (entry_rva - sec_start)
                break
    reloc_data = b''
    for off in lumie_relocs:
        reloc_data += struct.pack('<I', off)
    reloc_count = len(lumie_relocs)
    name_bytes = name.encode('ascii', errors='replace')[:23]
    name_bytes = name_bytes.ljust(24, b'\x00')
    hdr = struct.pack('<IIIIIIIIII',
        magic, entry_flat, code_size, 0, 0, reloc_count, 0, 0, 0, 0,
    )
    hdr += name_bytes
    reloc_off = 64 + code_size
    hdr = hdr[:16] + struct.pack('<I', reloc_off) + hdr[20:]
    os.makedirs(os.path.dirname(output_path) or '.', exist_ok=True)
    with open(output_path, 'wb') as f:
        f.write(hdr)
        f.write(bytes(code_blob))
        f.write(reloc_data)
    print(f"[SDK] Created {output_path}")
    print(f"     Magic: 0x{magic:08X} | Entry: 0x{entry_flat:X} | Code: {code_size} bytes | Relocs: {reloc_count}")


def build_module(crate_path, crate_name, magic, module_type, output_name=None, release=True):
    print(f"[SDK] Building {module_type}: {crate_name}")
    cargo_build(crate_path, release)
    a_file = find_a_file(crate_name)
    if not a_file:
        print(f"[SDK] ERROR: .a file not found for {crate_name}", file=sys.stderr)
        sys.exit(1)
    print(f"[SDK] Found archive: {a_file}")
    tmpdir = tempfile.mkdtemp(prefix="lumie_sdk_")
    try:
        dll_path = os.path.join(tmpdir, "temp.dll")
        link_to_pe_dll(a_file, dll_path)
        with open(dll_path, 'rb') as f:
            pe_data = f.read()
        if output_name is None:
            output_name = crate_name
        ext_map = {"app": ".lsh", "driver": ".ldrv", "kernel": ".lkrn"}
        ext = ext_map.get(module_type, ".lsh")
        out_dir = os.path.join(BUILD_DIR, "release", "system" if module_type != "driver" else "drivers")
        os.makedirs(out_dir, exist_ok=True)
        output_path = os.path.join(out_dir, f"{output_name}{ext}")
        convert_pe_to_lumie(pe_data, magic, output_name, output_path)
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)


def build_app(name, release=True):
    crate_path = os.path.join(PROJECT_ROOT, name)
    if not os.path.exists(os.path.join(crate_path, "Cargo.toml")):
        crate_path = os.path.join(PROJECT_ROOT, "app_" + name)
    if not os.path.exists(os.path.join(crate_path, "Cargo.toml")):
        print(f"[SDK] ERROR: Could not find crate for app '{name}'", file=sys.stderr)
        sys.exit(1)
    build_module(crate_path, name, MAGIC_LSH, "app", name, release)


def build_driver(name, release=True):
    crate_path = os.path.join(PROJECT_ROOT, name)
    if not os.path.exists(os.path.join(crate_path, "Cargo.toml")):
        crate_path = os.path.join(PROJECT_ROOT, "drv_" + name)
    if not os.path.exists(os.path.join(crate_path, "Cargo.toml")):
        print(f"[SDK] ERROR: Could not find crate for driver '{name}'", file=sys.stderr)
        sys.exit(1)
    build_module(crate_path, name, MAGIC_LDRV, "driver", name, release)


def build_kernel(release=True):
    crate_path = os.path.join(PROJECT_ROOT, "kernel")
    build_module(crate_path, "lumieos_kernel", MAGIC_LKRN, "kernel", "kernel", release)


def compile_lumiec(input_file, output_file=None):
    if output_file is None:
        output_file = os.path.splitext(input_file)[0] + ".sys"
    print(f"[SDK] LumieC compile: {input_file} -> {output_file}")
    cargo = find_cargo()
    cmd = [cargo, "build", "--release", "--target", CARGO_TARGET]
    print(f"[SDK] NOTE: LumieC compiler runs inside LumieOS. Use 'lumiec' command in the OS shell.")
    print(f"[SDK] Place {input_file} on the disk image and run: lumiec {os.path.basename(input_file)}")


def clean():
    target_dir = os.path.join(PROJECT_ROOT, "target")
    if os.path.exists(target_dir):
        shutil.rmtree(target_dir)
        print("[SDK] Cleaned target/")
    build_dir = os.path.join(BUILD_DIR, "release")
    if os.path.exists(build_dir):
        shutil.rmtree(build_dir)
        print("[SDK] Cleaned build/release/")


def main():
    if len(sys.argv) < 2:
        print(__doc__, file=sys.stderr)
        sys.exit(1)
    
    cmd = sys.argv[1].lower()
    release = "--debug" not in sys.argv
    
    if cmd == "app":
        if len(sys.argv) < 3:
            print("[SDK] Usage: sdk_build.py app <name>", file=sys.stderr)
            sys.exit(1)
        build_app(sys.argv[2], release)
    elif cmd == "driver":
        if len(sys.argv) < 3:
            print("[SDK] Usage: sdk_build.py driver <name>", file=sys.stderr)
            sys.exit(1)
        build_driver(sys.argv[2], release)
    elif cmd == "kernel":
        build_kernel(release)
    elif cmd == "lumiec":
        if len(sys.argv) < 3:
            print("[SDK] Usage: sdk_build.py lumiec <file.lc>", file=sys.stderr)
            sys.exit(1)
        output = sys.argv[3] if len(sys.argv) > 3 and sys.argv[2] == "-o" else None
        compile_lumiec(sys.argv[2], output)
    elif cmd == "clean":
        clean()
    elif cmd == "package":
        print("[SDK] Creating install package...")
        pkg_script = os.path.join(PROJECT_ROOT, "tools", "make_install_pkg.py")
        if os.path.exists(pkg_script):
            subprocess.run(["python", pkg_script], cwd=PROJECT_ROOT)
        else:
            print("[SDK] Package script not found", file=sys.stderr)
    else:
        print(f"[SDK] Unknown command: {cmd}", file=sys.stderr)
        print(__doc__, file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
