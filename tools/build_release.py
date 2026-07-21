#!/usr/bin/env python3
"""Build LumieOS release: convert .a -> .lkrn/.lsh and create install.pkg"""

import subprocess, os, sys, tempfile, shutil, glob, struct

sys.path.insert(0, os.path.dirname(__file__))
import a2lmod
import make_install_pkg

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
TARGET_DEPS = os.path.join(PROJECT_ROOT, 'target', 'x86_64-unknown-uefi', 'release', 'deps')
RELEASE_DIR = os.path.join(PROJECT_ROOT, 'build', 'release')

# Find lld-link
LLD = os.environ.get('LLD_LINK')
if LLD and os.path.exists(LLD):
    pass
else:
    LLD = None
    # Try /tmp/lld-link symlink first (Linux)
    if os.path.exists('/tmp/lld-link'):
        LLD = '/tmp/lld-link'
    else:
        # Try rust-lld from nightly toolchain
        import glob as globmod
        
        # First try lld-link directly (Windows naming)
        for p in globmod.glob(os.path.expanduser('~/.rustup/toolchains/nightly-*/lib/rustlib/*/bin/gcc-ld/lld-link')):
            if os.path.exists(p):
                LLD = p
                break
        
        # Then try ld.lld (Unix naming)
        if not LLD:
            for p in globmod.glob(os.path.expanduser('~/.rustup/toolchains/nightly-*/lib/rustlib/*/bin/gcc-ld/ld.lld')):
                if os.path.exists(p):
                    LLD = p
                    break
        
        # Try rust-lld as last resort
        if not LLD:
            for p in globmod.glob(os.path.expanduser('~/.rustup/toolchains/nightly-*/lib/rustlib/*/bin/rust-lld')):
                if os.path.exists(p):
                    LLD = p
                    break
        
        # Windows: search for lld-link.exe
        if not LLD:
            for root, dirs, files in os.walk(os.path.expanduser('~/.rustup')):
                for f in files:
                    if f.lower() == 'lld-link.exe':
                        LLD = os.path.join(root, f)
                        break
                if LLD:
                    break

if not LLD:
    print("ERROR: lld-link not found. Set LLD_LINK environment variable.", file=sys.stderr)
    print("Example: export LLD_LINK=$(find ~/.rustup -name 'lld-link' | head -1)", file=sys.stderr)
    sys.exit(1)

print(f"Using linker: {LLD}")

def latest_a(pattern):
    files = glob.glob(os.path.join(TARGET_DEPS, pattern))
    if not files:
        return None
    return max(files, key=os.path.getctime)

def find_builtins():
    files = glob.glob(os.path.join(TARGET_DEPS, 'libcompiler_builtins-*.rlib'))
    return files[0] if files else None

def link_and_convert(crate_name, a_file, output_path, magic, mod_name):
    builtins = find_builtins()
    if not builtins:
        print(f"ERROR: compiler_builtins not found for {crate_name}", file=sys.stderr)
        return False

    tmpdir = tempfile.mkdtemp(prefix=f'{crate_name}_')
    try:
        dll_path = os.path.join(tmpdir, f'{crate_name}.dll')

        # Use lld-link with Windows-style flags
        cmd = [
            LLD, '/dll', '/noentry', '/nodefaultlib',
            '/machine:x64', '/nologo',
            '/lldmingw',
            '/base:0',
            f'/out:{dll_path}',
            f'/wholearchive:{a_file}',
            builtins,
        ]
        print(f"  Linking {crate_name}...")
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"  Link failed: {result.stderr[:500]}", file=sys.stderr)
            return False

        with open(dll_path, 'rb') as f:
            pe_data = f.read()

        os.makedirs(os.path.dirname(output_path) or '.', exist_ok=True)
        a2lmod.convert_pe_to_lumie(pe_data, magic, mod_name, output_path)
        print(f"  Created {output_path}")
        return True
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)

def main():
    os.chdir(PROJECT_ROOT)

    # Ensure release dir structure
    os.makedirs(os.path.join(RELEASE_DIR, 'system'), exist_ok=True)
    os.makedirs(os.path.join(RELEASE_DIR, 'drivers'), exist_ok=True)
    os.makedirs(os.path.join(RELEASE_DIR, 'EFI', 'BOOT'), exist_ok=True)

    # Map: (crate_name, .a pattern, output_rel_path, magic_type, module_name)
    components = [
        ('kernel',   'liblumieos_kernel-*.a',   'system/kernel.lkrn',  'LKRN', 'LumieOS Kernel'),
        ('shell',    'liblumieos_shell-*.a',    'system/shell.lsh',    'LSH',  'LumieOS Shell'),
        ('editor',   'liblumieos_editor-*.a',   'system/editor.lsh',   'LSH',  'LumieOS Editor'),
        ('desktop',  'liblumieos_desktop-*.a',  'system/desktop.lsh',  'LSH',  'LumieOS Desktop'),
    ]

    success = True
    for crate_name, a_pattern, rel_path, magic_type, mod_name in components:
        print(f"\nProcessing {crate_name}...")
        a_file = latest_a(a_pattern)
        if not a_file:
            print(f"  SKIP: .a file not found for {crate_name}", file=sys.stderr)
            success = False
            continue
        output = os.path.join(RELEASE_DIR, rel_path)
        magic = a2lmod.MAGIC_LKRN if magic_type == 'LKRN' else (
            a2lmod.MAGIC_LSH if magic_type == 'LSH' else (
            a2lmod.MAGIC_LDRV if magic_type == 'LDRV' else a2lmod.MAGIC_SYS))
        if not link_and_convert(crate_name, a_file, output, magic, mod_name):
            success = False

    # Driver components
    driver_components = [
        ('drv_kbd',   'liblumieos_drv_kbd-*.a',   'drivers/kbd.ldrv',    'LDRV', 'Keyboard Driver'),
        ('drv_mouse', 'liblumieos_drv_mouse-*.a',  'drivers/mouse.ldrv',  'LDRV', 'Mouse Driver'),
        ('drv_fs',    'liblumieos_drv_fs-*.a',     'drivers/fs.ldrv',     'LDRV', 'Filesystem Driver'),
    ]
    for crate_name, a_pattern, rel_path, magic_type, mod_name in driver_components:
        print(f"\nProcessing {crate_name}...")
        a_file = latest_a(a_pattern)
        if not a_file:
            print(f"  SKIP: .a file not found for {crate_name}", file=sys.stderr)
            continue
        output = os.path.join(RELEASE_DIR, rel_path)
        magic = a2lmod.MAGIC_LDRV
        link_and_convert(crate_name, a_file, output, magic, mod_name)

    # Copy BOOTX64.EFI
    print("\nCopying BOOTX64.EFI...")
    efi_src = os.path.join(PROJECT_ROOT, 'target', 'x86_64-unknown-uefi', 'release', 'lumieos-loader.efi')
    if not os.path.exists(efi_src):
        efi_src = os.path.join(PROJECT_ROOT, 'target', 'x86_64-unknown-uefi', 'release', 'BOOTX64.EFI')
    if os.path.exists(efi_src):
        efi_dst = os.path.join(RELEASE_DIR, 'EFI', 'BOOT', 'BOOTX64.EFI')
        shutil.copy2(efi_src, efi_dst)
        print(f"  Copied {efi_src} -> {efi_dst}")
    else:
        print(f"  WARNING: BOOTX64.EFI not found at {efi_src}", file=sys.stderr)

    # Create install.pkg
    print("\nCreating install.pkg...")
    pkg_output = os.path.join(PROJECT_ROOT, 'build', 'install.pkg')
    make_install_pkg.make_install_pkg(RELEASE_DIR, pkg_output, compress=True)

    print("\nDone!")
    if not success:
        print("Some components failed (see above).")
        sys.exit(1)

if __name__ == '__main__':
    main()
