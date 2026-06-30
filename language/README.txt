LumieC Language Reference
=========================
LumieC is a C-like compiled language for LumieOS.
Compiler: lumiec <input.lc> [output.sys]
Loader:   sysload <path.sys>

Syntax Notes
------------
- Types: void, char, u8, u16, u32, u64, i8, i16, i32, i64, int
- Pointers, arrays, function calls work like C
- No structs, no OOP, no floats
- Entry point must be: int lumiec_main(void)  /* @public */

Decompilation Directives (comment-based)
------------------------------------------
/* @public */   - Keep visible in decompiler output
/* @hide */     - Hide this function/section
/* @obfuscate */ - Obfuscate this section
/* @nomem */    - No memory analysis on this block
/* @strip */    - Remove from decompiler output entirely

Syscalls
---------
syscall0(n)             - void syscall
syscall1(n, a1)         - 1 argument
syscall2(n, a1, a2)     - 2 arguments
syscall3(n, a1, a2, a3) - 3 arguments
syscall4(n, a1, a2, a3, a4)
syscall5(n, a1, a2, a3, a4, a5)
syscall6(n, a1, a2, a3, a4, a5, a6)

Common syscall numbers:
  0x01 - sys_write(const char*)
  0x10 - get_cursor_pos(i64*x, i64*y)
  0x20 - draw_pixel(i64 x, i64 y, u32 color)

Built-in Functions
-------------------
void *kmalloc(i64 size)
void kfree(void *ptr)
void memset(void *dst, i64 val, i64 size)
void memcpy(void *dst, void *src, i64 size)
i64 strlen(const char *s)
i64 strcmp(const char *a, const char *b)
void strcpy(char *dst, const char *src)

Output Format
--------------
.sys module with:
  - Header (magic 0x01535953, entry, size)
  - .text section (x86-64 machine code)
  - Relocation table
