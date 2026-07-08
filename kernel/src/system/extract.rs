use core::ptr;
use crate::console::terminal;
use crate::fs;

/* Bit reader */
struct BitReader {
    data: *const u8,
    size: u32,
    buf: u32,
    bits: i32,
    pos: u32,
}

impl BitReader {
    fn new(data: *const u8, size: u32) -> Self {
        BitReader { data, size, buf: 0, bits: 0, pos: 0 }
    }

    fn read(&mut self, n: i32) -> u32 {
        let n = if n > 31 { 31 } else { n };
        while self.bits < n && self.pos < self.size {
            self.buf |= unsafe { *self.data.add(self.pos as usize) as u32 } << self.bits;
            self.pos += 1;
            self.bits += 8;
        }
        let r = self.buf & ((1u32 << n) - 1);
        self.buf >>= n;
        self.bits -= n;
        r
    }

    fn align(&mut self) {
        self.bits = 0;
        self.buf = 0;
    }
}

/* Huffman tree */
const HT_NODES: usize = 1024;

#[derive(Clone, Copy)]
struct HtNode {
    left: u16,
    right: u16,
    symbol: u16,
    is_leaf: u8,
}

struct HuffmanTree {
    n: [HtNode; HT_NODES],
    num: i32,
}

impl HuffmanTree {
    fn new() -> Self {
        HuffmanTree {
            n: [HtNode { left: 0xFFFF, right: 0xFFFF, symbol: 0, is_leaf: 0 }; HT_NODES],
            num: 0,
        }
    }

    fn new_node(&mut self) -> i32 {
        if self.num >= HT_NODES as i32 {
            return -1;
        }
        let idx = self.num as usize;
        self.num += 1;
        self.n[idx] = HtNode { left: 0xFFFF, right: 0xFFFF, symbol: 0, is_leaf: 0 };
        idx as i32
    }

    fn build(&mut self, lens: &[u16]) -> i32 {
        self.num = 0;
        let root = self.new_node();
        if root < 0 {
            return -1;
        }
        let nsym = lens.len();

        let mut cnt = [0u16; 16];
        for i in 0..nsym {
            if lens[i] > 0 && lens[i] < 16 {
                cnt[lens[i] as usize] += 1;
            }
        }

        let mut code = [0u16; 16];
        let mut next: u16 = 0;
        for b in 1..16 {
            code[b] = next;
            next = (next + cnt[b]) << 1;
        }

        for sym in 0..nsym {
            let len = lens[sym] as usize;
            if len == 0 {
                continue;
            }
            let c = code[len];
            code[len] += 1;
            let mut node = root as usize;
            for b in (0..len).rev() {
                if (c >> b) & 1 != 0 {
                    if self.n[node].right == 0xFFFF {
                        let n2 = self.new_node();
                        if n2 < 0 {
                            return -1;
                        }
                        self.n[node].right = n2 as u16;
                    }
                    node = self.n[node].right as usize;
                } else {
                    if self.n[node].left == 0xFFFF {
                        let n2 = self.new_node();
                        if n2 < 0 {
                            return -1;
                        }
                        self.n[node].left = n2 as u16;
                    }
                    node = self.n[node].left as usize;
                }
            }
            self.n[node].is_leaf = 1;
            self.n[node].symbol = sym as u16;
        }
        root
    }

    fn decode(&self, br: &mut BitReader, root: i32) -> i32 {
        let mut node = root as usize;
        while self.n[node].is_leaf == 0 {
            let bit = br.read(1) as i32;
            if bit == 0 {
                if self.n[node].left == 0xFFFF {
                    return -1;
                }
                node = self.n[node].left as usize;
            } else {
                if self.n[node].right == 0xFFFF {
                    return -1;
                }
                node = self.n[node].right as usize;
            }
        }
        self.n[node].symbol as i32
    }
}

/* Length/distance tables */
const LEN_BASE: [u16; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEN_EXTRA: [u16; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DST_BASE: [u16; 30] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577];
const DST_EXTRA: [u16; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

/* CRC32 */
static mut CRC32_TAB: [u32; 256] = [0u32; 256];
static mut CRC32_OK: bool = false;

unsafe fn crc32_init() {
    for i in 0..256u32 {
        let mut c = i;
        for _ in 0..8 {
            c = if (c & 1) != 0 { (c >> 1) ^ 0xEDB88320 } else { c >> 1 };
        }
        CRC32_TAB[i as usize] = c;
    }
    CRC32_OK = true;
}

unsafe fn crc32_calc(mut crc: u32, d: *const u8, sz: u32) -> u32 {
    if !CRC32_OK {
        crc32_init();
    }
    for i in 0..sz as usize {
        crc = CRC32_TAB[((crc ^ (*d.add(i) as u32)) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc
}

/* DEFLATE */
unsafe fn deflate_decompress(br: &mut BitReader, out: *mut u8, out_max: u32, out_size: &mut u32) -> i32 {
    let mut pos: u32 = 0;
    let mut bfinal = 0;

    while bfinal == 0 {
        bfinal = br.read(1) as i32;
        let btype = br.read(2) as i32;

        if btype == 0 {
            br.align();
            let len = br.read(16) as u16;
            let nlen = br.read(16) as u16;
            if (len ^ 0xFFFF) != nlen {
                terminal::term_writeln(b"DEFLATE: NLEN mismatch!\0" as *const u8);
                return -1;
            }
            for _ in 0..(len as u32) {
                if br.pos < br.size && pos < out_max {
                    *out.add(pos as usize) = *br.data.add(br.pos as usize);
                    pos += 1;
                    br.pos += 1;
                }
            }
            br.buf = 0;
            br.bits = 0;
        } else if btype == 1 || btype == 2 {
            let mut tl: HuffmanTree = HuffmanTree::new();
            let mut td: HuffmanTree = HuffmanTree::new();
            let mut ll = [0u16; 288];
            let mut dl = [0u16; 32];

            if btype == 1 {
                for i in 0..144 { ll[i] = 8; }
                for i in 144..256 { ll[i] = 9; }
                for i in 256..280 { ll[i] = 7; }
                for i in 280..288 { ll[i] = 8; }
                for i in 0..32 { dl[i] = 5; }
            } else {
                let hlit = br.read(5) as usize + 257;
                let hdist = br.read(5) as usize + 1;
                let hclen = br.read(4) as usize + 4;
                let co: [u8; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
                let mut cl = [0u16; 19];
                let mut hcl = HuffmanTree::new();
                for i in 0..hclen {
                    cl[co[i] as usize] = br.read(3) as u16;
                }
                let cl_root = hcl.build(&cl);
                if cl_root < 0 { return -1; }

                let mut buf = [0u16; 320];
                let mut bi: usize = 0;
                let total = hlit + hdist;
                while bi < total {
                    let s = hcl.decode(br, cl_root);
                    if s < 0 { return -1; }
                    if s < 16 { buf[bi] = s as u16; bi += 1; }
                    else if s == 16 {
                        let r = br.read(2) + 3;
                        let p = if bi > 0 { buf[bi - 1] } else { 0 };
                        for _ in 0..r {
                            if bi < total { buf[bi] = p; bi += 1; }
                        }
                    } else if s == 17 {
                        let r = br.read(3) + 3;
                        for _ in 0..r {
                            if bi < total { buf[bi] = 0; bi += 1; }
                        }
                    } else if s == 18 {
                        let r = br.read(7) + 11;
                        for _ in 0..r {
                            if bi < total { buf[bi] = 0; bi += 1; }
                        }
                    }
                }
                for i in 0..hlit { ll[i] = buf[i]; }
                for i in 0..hdist { dl[i] = buf[hlit + i]; }
            }

            let lr = tl.build(&ll);
            let dr = td.build(&dl);
            if lr < 0 || dr < 0 { return -1; }

            loop {
                let sym = tl.decode(br, lr);
                if sym < 0 { return -1; }
                if sym < 256 {
                    if pos >= out_max { return -1; }
                    *out.add(pos as usize) = sym as u8;
                    pos += 1;
                } else if sym == 256 { break; }
                else {
                    let li = sym - 257;
                    if li < 0 || li >= 29 { return -1; }
                    let len = LEN_BASE[li as usize] as u32 + br.read(LEN_EXTRA[li as usize] as i32);
                    let ds = td.decode(br, dr);
                    if ds < 0 || ds >= 30 { return -1; }
                    let dist = DST_BASE[ds as usize] as u32 + br.read(DST_EXTRA[ds as usize] as i32);
                    for _ in 0..len {
                        if pos >= out_max || pos < dist { return -1; }
                        *out.add(pos as usize) = *out.add((pos - dist) as usize);
                        pos += 1;
                    }
                }
            }
        } else {
            return -1;
        }
    }
    *out_size = pos;
    0
}

/* Gzip */
unsafe fn gzip_decompress(inp: *const u8, in_sz: u32, out: *mut *mut u8, out_sz: &mut u32) -> i32 {
    if in_sz < 18 || *inp != 0x1F || *inp.add(1) != 0x8B || *inp.add(2) != 8 {
        return -1;
    }
    let flg = *inp.add(3);
    let mut hdr: u32 = 10;
    if flg & 0x04 != 0 {
        if hdr + 2 > in_sz { return -1; }
        hdr += 2 + (*inp.add(hdr as usize) as u32 | (*inp.add(hdr as usize + 1) as u32) << 8);
    }
    if flg & 0x08 != 0 {
        while hdr < in_sz && *inp.add(hdr as usize) != 0 { hdr += 1; }
        hdr += 1;
    }
    if flg & 0x10 != 0 {
        while hdr < in_sz && *inp.add(hdr as usize) != 0 { hdr += 1; }
        hdr += 1;
    }
    if flg & 0x02 != 0 { hdr += 2; }
    if hdr >= in_sz { return -1; }

    let mut max_out64 = (in_sz as u64) * 4;
    if max_out64 < 65536 { max_out64 = 65536; }
    if max_out64 > 256 * 1024 * 1024 { max_out64 = 256 * 1024 * 1024; }
    let max_out = max_out64 as u32;

    let dec_buf = crate::mm::alloc(max_out as u64);
    if dec_buf.is_null() { return -1; }

    let mut br = BitReader::new(inp.add(hdr as usize), in_sz - hdr - 8);
    let mut dec: u32 = 0;
    let ret = deflate_decompress(&mut br, dec_buf, max_out, &mut dec);
    if ret < 0 {
        crate::mm::free(dec_buf);
        return -1;
    }

    let tr = (in_sz - 8) as usize;
    let hdr_u = hdr as usize;
    if tr >= hdr_u {
        let ecrc = (*inp.add(tr) as u32)
            | ((*inp.add(tr + 1) as u32) << 8)
            | ((*inp.add(tr + 2) as u32) << 16)
            | ((*inp.add(tr + 3) as u32) << 24);
        let acrc = crc32_calc(0xFFFFFFFF, dec_buf, dec) ^ 0xFFFFFFFF;
        if acrc != ecrc {
            terminal::term_writeln(b" CRC mismatch (ignored)\0" as *const u8);
        }
    }

    *out = dec_buf;
    *out_sz = dec;
    0
}

/* Tar */
#[repr(C)]
struct TarHdr {
    name: [u8; 100],
    mode: [u8; 8],
    uid: [u8; 8],
    gid: [u8; 8],
    size: [u8; 12],
    mtime: [u8; 12],
    chksum: [u8; 8],
    typeflag: u8,
    linkname: [u8; 100],
    magic: [u8; 6],
    version: [u8; 2],
    uname: [u8; 32],
    gname: [u8; 32],
    devmajor: [u8; 8],
    devminor: [u8; 8],
    prefix: [u8; 155],
    pad: [u8; 12],
}

fn parse_oct(s: &[u8]) -> u32 {
    let mut v: u32 = 0;
    for i in 0..s.len() {
        if s[i] >= b'0' && s[i] <= b'7' {
            v = v * 8 + (s[i] - b'0') as u32;
        }
    }
    v
}

unsafe fn tar_extract(data: *const u8, size: u32) -> i32 {
    let mut off: u32 = 0;
    let mut cnt: i32 = 0;
    while off + 512 <= size {
        let h = &*(data.add(off as usize) as *const TarHdr);
        if h.name[0] == 0 {
            break;
        }
        let mut valid = h.magic[..5] == *b"ustar";
        if !valid {
            if h.name[0] >= 0x20 && h.name[0] < 0x7F {
                valid = true;
            } else {
                break;
            }
        }
        let fsz = parse_oct(&h.size);
        let mut path: [u8; 256] = [0u8; 256];
        let mut _plen: usize = 0;
        if h.prefix[0] != 0 {
            let mut ppl = 0;
            while ppl < 155 && h.prefix[ppl] != 0 { ppl += 1; }
            let mut pnl = 0;
            while pnl < 100 && h.name[pnl] != 0 { pnl += 1; }
            if ppl + 1 + pnl > 255 { break; }
            path[..ppl].copy_from_slice(&h.prefix[..ppl]);
            _plen = ppl;
            path[_plen] = b'/';
            _plen += 1;
            path[_plen.._plen + pnl].copy_from_slice(&h.name[..pnl]);
            _plen += pnl;
        } else {
            let mut pnl = 0;
            while pnl < 100 && h.name[pnl] != 0 { pnl += 1; }
            path[..pnl].copy_from_slice(&h.name[..pnl]);
            _plen = pnl;
        }
        path[_plen] = 0;

        terminal::term_set_fg(if h.typeflag == b'5' { 0x55FFFF } else { 0x00FF00 });
        if h.typeflag == b'5' {
            terminal::term_write(b" [DIR] \0" as *const u8);
        } else {
            terminal::term_write(b" [FILE] \0" as *const u8);
        }
        terminal::term_set_fg(0xFFFFFF);
        terminal::term_writeln(path.as_ptr());

        let doff = off + 512;
        if (h.typeflag == b'0' || h.typeflag == 0) && fsz > 0 {
            if doff + fsz <= size {
                fs::fat32::write_file(path.as_ptr() as *const u8, data.add(doff as usize) as *const u8, fsz);
            }
        }
        off += 512 + ((fsz + 511) / 512) * 512;
        cnt += 1;
    }
    cnt
}

/* Public API */
pub unsafe fn extract_gzip_tar(filename: *const u8) -> i32 {
    if filename.is_null() {
        terminal::term_set_fg(0xFF4444);
        terminal::term_writeln(b"Usage: extract <file>\0" as *const u8);
        terminal::term_set_fg(0xAAAAAA);
        return -1;
    }
    let fname = crate::system::util::lumie_str_from_ptr(filename);

    let fsz = fs::fat32::get_file_size(fname.as_ptr() as *const u8);
    if fsz <= 0 {
        terminal::term_set_fg(0xFF4444);
        terminal::term_write(b"File not found: \0" as *const u8);
        terminal::term_writeln(filename);
        terminal::term_set_fg(0xAAAAAA);
        return -1;
    }

    let mut buf: [u8; 64] = [0u8; 64];
    crate::system::util::lumie_itoa(fsz as i64, buf.as_mut_ptr(), 10);
    terminal::term_write(b"Reading \0" as *const u8);
    terminal::term_write(buf.as_ptr());
    terminal::term_writeln(b" bytes\0" as *const u8);

    let data = crate::mm::alloc(fsz as u64);
    if data.is_null() {
        terminal::term_set_fg(0xFF4444);
        terminal::term_writeln(b"Out of memory\0" as *const u8);
        terminal::term_set_fg(0xAAAAAA);
        return -1;
    }

    if fs::fat32::read_file(fname.as_ptr() as *const u8, data, fsz as u32) != fsz {
        crate::mm::free(data);
        terminal::term_set_fg(0xFF4444);
        terminal::term_writeln(b"Read error\0" as *const u8);
        terminal::term_set_fg(0xAAAAAA);
        return -1;
    }

    if *data == 0x1F && *data.add(1) == 0x8B {
        terminal::term_writeln(b"Format: gzip\0" as *const u8);
        let mut dec: *mut u8 = ptr::null_mut();
        let mut dec_sz: u32 = 0;
        let ret = gzip_decompress(data, fsz as u32, &mut dec, &mut dec_sz);
        crate::mm::free(data);
        if ret < 0 || dec.is_null() {
            terminal::term_set_fg(0xFF4444);
            terminal::term_writeln(b"Decompression failed\0" as *const u8);
            terminal::term_set_fg(0xAAAAAA);
            return -1;
        }
        let mut dbuf: [u8; 64] = [0u8; 64];
        crate::system::util::lumie_itoa(dec_sz as i64, dbuf.as_mut_ptr(), 10);
        terminal::term_write(b"Decompressed: \0" as *const u8);
        terminal::term_write(dbuf.as_ptr());
        terminal::term_writeln(b" bytes\0" as *const u8);

        let cnt = tar_extract(dec, dec_sz);
        crate::mm::free(dec);
        let mut cbuf: [u8; 64] = [0u8; 64];
        crate::system::util::lumie_itoa(cnt as i64, cbuf.as_mut_ptr(), 10);
        terminal::term_write(cbuf.as_ptr());
        terminal::term_writeln(b" entries extracted\0" as *const u8);
        return 0;
    }

    let xz_magic = [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00];
    let hdr = core::slice::from_raw_parts(data, 6);
    if hdr == &xz_magic {
        terminal::term_writeln(b"Format: XZ (not implemented)\0" as *const u8);
        crate::mm::free(data);
        terminal::term_set_fg(0xFF4444);
        terminal::term_writeln(b"XZ decompression not available\0" as *const u8);
        terminal::term_set_fg(0xAAAAAA);
        return -1;
    }

    if crate::system::util::lumie_str_from_ptr(filename).contains(".tar") {
        terminal::term_writeln(b"Format: tar\0" as *const u8);
        let cnt = tar_extract(data, fsz as u32);
        crate::mm::free(data);
        let mut cbuf: [u8; 64] = [0u8; 64];
        crate::system::util::lumie_itoa(cnt as i64, cbuf.as_mut_ptr(), 10);
        terminal::term_write(cbuf.as_ptr());
        terminal::term_writeln(b" entries extracted\0" as *const u8);
        return 0;
    }

    crate::mm::free(data);
    terminal::term_set_fg(0xFF4444);
    terminal::term_writeln(b"Unknown format\0" as *const u8);
    terminal::term_set_fg(0xAAAAAA);
    -1
}
