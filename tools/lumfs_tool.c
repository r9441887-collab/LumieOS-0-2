/*
 * lumfs_tool.c - LumFS reader/mounter for Windows (pure Win32, no deps)
 *
 * Reads a LumFS partition from USB/image and either:
 *   - Lists/extracts files via CLI
 *   - "Mounts" via subst: extracts to temp dir, maps drive letter
 *
 * Build:
 *   gcc -o lumfs_tool.exe lumfs_tool.c
 *
 * Usage:
 *   lumfs_tool.exe info       <device> [offset]
 *   lumfs_tool.exe list       <device> [offset]
 *   lumfs_tool.exe cat        <device> <path> [offset]
 *   lumfs_tool.exe extract    <device> <path> <output> [offset]
 *   lumfs_tool.exe allextract <device> <path> <dest_dir> [offset]
 *   lumfs_tool.exe mount      <device> <letter:> [offset]
 *   lumfs_tool.exe unmount    <letter:>
 *
 * Examples:
 *   lumfs_tool.exe list \\.\PhysicalDrive3
 *   lumfs_tool.exe mount \\.\PhysicalDrive3 Z:
 *   lumfs_tool.exe unmount Z:
 *
 * Find USB: wmic diskdrive list brief
 */

#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#define LUM_MAGIC           0x4C554D32
#define LUM_VERSION         2
#define LUM_CELL_SIZE       4096
#define LUM_SECT_PER_CELL   (LUM_CELL_SIZE / 512)
#define LUM_HDR_SIZE        48
#define LUM_DATA_SIZE       (LUM_CELL_SIZE - LUM_HDR_SIZE)
#define LUM_ROOT_CHAIN      1

#define LUM_TYPE_FREE       0
#define LUM_TYPE_DEAD       1
#define LUM_TYPE_FILE_HDR   2
#define LUM_TYPE_FILE_DATA  3
#define LUM_TYPE_DIR        4

#pragma pack(push, 1)
typedef struct {
    uint32_t magic;
    uint16_t version;
    uint16_t flags;
    uint64_t total_cells;
    uint64_t write_ptr;
    uint64_t gc_ptr;
    uint32_t root_dir_cell;
    uint32_t file_count;
    uint64_t free_count;
    uint8_t  volume_label[32];
    uint32_t checksum;
} LumSuperblock;

typedef struct {
    uint32_t magic;
    uint32_t chain_id;
    uint32_t seq;
    uint32_t next_cell;
    uint8_t  cell_type;
    uint8_t  flags;
    uint16_t name_hash;
    uint16_t data_len;
    uint16_t reserved;
    uint32_t crc;
    uint32_t parent_chain;
    uint32_t total_size;
    uint32_t created;
    uint32_t modified;
} LumCellHdr;
#pragma pack(pop)

static HANDLE   g_disk   = INVALID_HANDLE_VALUE;
static uint64_t g_offset = 0;
static int      g_is_file = 0;
static uint8_t *g_cell_buf = NULL;

/* ------------------------------------------------------------------ */
/* Disk I/O                                                           */
/* ------------------------------------------------------------------ */

static int disk_read_sectors(uint32_t lba, uint32_t count, void *buf) {
    uint64_t pos = ((uint64_t)lba + g_offset) * 512ULL;
    LARGE_INTEGER li;
    DWORD got = 0;
    li.QuadPart = (LONGLONG)pos;
    if (!SetFilePointerEx(g_disk, li, NULL, FILE_BEGIN)) return -1;
    if (!ReadFile(g_disk, buf, count * 512U, &got, NULL)) return -1;
    if (got != count * 512U) return -1;
    return 0;
}

static int read_cell(uint32_t cell_id, void *buf) {
    return disk_read_sectors(cell_id * LUM_SECT_PER_CELL, LUM_SECT_PER_CELL, buf);
}

static int read_hdr(uint32_t cell_id, LumCellHdr *hdr) {
    if (read_cell(cell_id, g_cell_buf) != 0) return -1;
    memcpy(hdr, g_cell_buf, sizeof(LumCellHdr));
    return 0;
}

static int read_sb(LumSuperblock *sb) {
    if (read_cell(0, g_cell_buf) != 0) return -1;
    memcpy(sb, g_cell_buf, sizeof(LumSuperblock));
    return 0;
}

/* ------------------------------------------------------------------ */
/* Chain helpers                                                      */
/* ------------------------------------------------------------------ */

static uint32_t find_latest_in_chain(uint32_t chain_id, uint8_t cell_type) {
    LumSuperblock sb;
    if (read_sb(&sb) != 0) return 0;
    uint32_t best_cell = 0, best_seq = 0;
    for (uint64_t pos = 1; pos < sb.write_ptr; pos++) {
        LumCellHdr h;
        if (read_hdr((uint32_t)pos, &h) != 0) break;
        if (h.magic == LUM_MAGIC && h.chain_id == chain_id &&
            h.cell_type == cell_type && h.flags == 0 && h.seq >= best_seq) {
            best_cell = (uint32_t)pos;
            best_seq = h.seq;
        }
    }
    return best_cell;
}

/* ------------------------------------------------------------------ */
/* Directory operations                                               */
/* ------------------------------------------------------------------ */

typedef struct {
    uint32_t child_chain;
    char     name[256];
    int      is_dir;
    uint32_t size;
} LumDirEntry;

static int list_dir_entries(uint32_t parent_chain, LumDirEntry *entries, int max_entries) {
    LumSuperblock sb;
    if (read_sb(&sb) != 0) return -1;
    int count = 0;
    for (uint64_t pos = 1; pos < sb.write_ptr && count < max_entries; pos++) {
        LumCellHdr h;
        if (read_hdr((uint32_t)pos, &h) != 0) break;
        if (h.magic != LUM_MAGIC || h.chain_id != parent_chain ||
            h.cell_type != LUM_TYPE_DIR || h.flags != 0)
            continue;
        if (read_cell((uint32_t)pos, g_cell_buf) != 0) break;
        uint8_t *data = g_cell_buf + LUM_HDR_SIZE;
        uint32_t off = 0;
        while (off + 8 <= h.data_len && count < max_entries) {
            uint32_t child = *(uint32_t *)(data + off);
            uint16_t nlen  = *(uint16_t *)(data + off + 4);
            uint8_t  isdir = *(uint8_t  *)(data + off + 6);
            if (off + 8 + nlen > h.data_len) break;
            if (nlen >= 256) goto skip_entry;
            {
                LumDirEntry *e = &entries[count];
                e->child_chain = child;
                e->is_dir = isdir;
                memcpy(e->name, data + off + 8, nlen);
                e->name[nlen] = 0;
                e->size = 0;
                if (!isdir) {
                    uint32_t fc = find_latest_in_chain(child, LUM_TYPE_FILE_HDR);
                    if (fc != 0) {
                        LumCellHdr fh;
                        if (read_hdr(fc, &fh) == 0) e->size = fh.total_size;
                    }
                }
                count++;
            }
        skip_entry:
            off += 8 + nlen;
            off = (off + 3) & ~3u;
        }
    }
    return count;
}

static uint32_t dir_find_entry_in_cells(uint32_t parent_chain, const char *name) {
    LumSuperblock sb;
    if (read_sb(&sb) != 0) return 0;
    for (uint64_t pos = 1; pos < sb.write_ptr; pos++) {
        LumCellHdr h;
        if (read_hdr((uint32_t)pos, &h) != 0) break;
        if (h.magic != LUM_MAGIC || h.chain_id != parent_chain ||
            h.cell_type != LUM_TYPE_DIR || h.flags != 0)
            continue;
        if (read_cell((uint32_t)pos, g_cell_buf) != 0) break;
        uint8_t *data = g_cell_buf + LUM_HDR_SIZE;
        uint32_t off = 0;
        while (off + 8 <= h.data_len) {
            uint32_t child = *(uint32_t *)(data + off);
            uint16_t nlen  = *(uint16_t *)(data + off + 4);
            if (off + 8 + nlen > h.data_len) break;
            char ename[256];
            memcpy(ename, data + off + 8, nlen);
            ename[nlen] = 0;
            if (strcmp(ename, name) == 0 && child != 0) return child;
            off += 8 + nlen;
            off = (off + 3) & ~3u;
        }
    }
    return 0;
}

/* ------------------------------------------------------------------ */
/* Path resolution                                                    */
/* ------------------------------------------------------------------ */

static uint32_t resolve_path(const char *path) {
    if (!path || *path == 0 || (path[0] == '/' && path[1] == 0))
        return LUM_ROOT_CHAIN;
    uint32_t current = LUM_ROOT_CHAIN;
    const char *p = path;
    while (*p == '/') p++;
    while (*p) {
        char comp[256];
        int i = 0;
        while (*p && *p != '/' && i < 255) comp[i++] = *p++;
        comp[i] = 0;
        while (*p == '/') p++;
        if (strcmp(comp, ".") == 0) continue;
        if (strcmp(comp, "..") == 0) {
            uint32_t hc = find_latest_in_chain(current, LUM_TYPE_DIR);
            if (hc == 0) hc = find_latest_in_chain(current, LUM_TYPE_FILE_HDR);
            if (hc != 0) {
                LumCellHdr h;
                if (read_hdr(hc, &h) == 0) current = h.parent_chain;
            }
            continue;
        }
        uint32_t f = dir_find_entry_in_cells(current, comp);
        if (f == 0) return 0;
        current = f;
    }
    return current;
}

/* ------------------------------------------------------------------ */
/* File info                                                          */
/* ------------------------------------------------------------------ */

typedef struct {
    uint32_t chain_id;
    int      is_dir;
    uint32_t size;
    uint32_t created;
    uint32_t modified;
} LumEntryInfo;

static int get_chain_info(uint32_t chain_id, LumEntryInfo *info) {
    memset(info, 0, sizeof(*info));
    info->chain_id = chain_id;
    uint32_t dc = find_latest_in_chain(chain_id, LUM_TYPE_DIR);
    if (dc != 0) {
        LumCellHdr h;
        if (read_hdr(dc, &h) == 0) {
            info->is_dir = 1;
            info->created = h.created;
            info->modified = h.modified;
            return 0;
        }
    }
    uint32_t fc = find_latest_in_chain(chain_id, LUM_TYPE_FILE_HDR);
    if (fc != 0) {
        LumCellHdr h;
        if (read_hdr(fc, &h) == 0) {
            info->is_dir = 0;
            info->size = h.total_size;
            info->created = h.created;
            info->modified = h.modified;
            return 0;
        }
    }
    return -1;
}

/* ------------------------------------------------------------------ */
/* Read file content                                                  */
/* ------------------------------------------------------------------ */

static int64_t read_file_content(uint32_t chain_id, uint8_t *out, uint64_t max_size) {
    uint32_t hdr_cell = find_latest_in_chain(chain_id, LUM_TYPE_FILE_HDR);
    if (hdr_cell == 0) return -1;
    LumCellHdr hdr;
    if (read_hdr(hdr_cell, &hdr) != 0) return -1;
    uint32_t total = hdr.total_size;
    uint32_t to_read = (uint32_t)(total < max_size ? total : max_size);
    if (to_read == 0) return 0;
    uint32_t total_read = 0;
    if (read_cell(hdr_cell, g_cell_buf) != 0) return -1;
    uint32_t hdr_inline = hdr.data_len;
    if (hdr_inline > LUM_DATA_SIZE) hdr_inline = LUM_DATA_SIZE;
    if (hdr_inline > to_read) hdr_inline = to_read;
    if (hdr_inline > 0) memcpy(out, g_cell_buf + LUM_HDR_SIZE, hdr_inline);
    total_read = hdr_inline;
    uint32_t next = hdr.next_cell;
    while (total_read < to_read && next != 0) {
        LumCellHdr dhdr;
        if (read_hdr(next, &dhdr) != 0) break;
        if (dhdr.cell_type != LUM_TYPE_FILE_DATA || dhdr.flags != 0) break;
        if (read_cell(next, g_cell_buf) != 0) break;
        uint32_t chunk = dhdr.data_len;
        if (chunk > LUM_DATA_SIZE) chunk = LUM_DATA_SIZE;
        if (chunk > to_read - total_read) chunk = to_read - total_read;
        memcpy(out + total_read, g_cell_buf + LUM_HDR_SIZE, chunk);
        total_read += chunk;
        next = dhdr.next_cell;
    }
    return (int64_t)total_read;
}

/* ================================================================== */
/* Commands                                                           */
/* ================================================================== */

static void cmd_info(void) {
    LumSuperblock sb;
    if (read_sb(&sb) != 0) { fprintf(stderr, "Error: cannot read superblock\n"); return; }
    if (sb.magic != LUM_MAGIC) {
        fprintf(stderr, "Error: bad magic 0x%08X (expected 0x%08X)\n", sb.magic, LUM_MAGIC); return;
    }
    if (sb.version != LUM_VERSION) {
        fprintf(stderr, "Error: version %u (expected %u)\n", sb.version, LUM_VERSION); return;
    }
    char label[33];
    memcpy(label, sb.volume_label, 32); label[32] = 0;
    for (int i = 31; i >= 0 && label[i] == ' '; i--) label[i] = 0;
    printf("=== LumFS Superblock ===\n");
    printf("  Volume label:  %s\n", label);
    printf("  Version:       %u\n", sb.version);
    printf("  Total cells:   %llu\n", (unsigned long long)sb.total_cells);
    printf("  Write pointer: %llu\n", (unsigned long long)sb.write_ptr);
    printf("  GC pointer:    %llu\n", (unsigned long long)sb.gc_ptr);
    printf("  Root dir cell: %u\n", sb.root_dir_cell);
    printf("  File count:    %u\n", sb.file_count);
    printf("  Free cells:    %llu\n", (unsigned long long)sb.free_count);
    printf("  Cell size:     %d\n", LUM_CELL_SIZE);
    printf("  Checksum:      0x%08X\n", sb.checksum);
}

static void list_recursive(uint32_t chain_id, const char *prefix, int depth) {
    if (depth > 32) return;
    LumDirEntry entries[512];
    int count = list_dir_entries(chain_id, entries, 512);
    for (int i = 0; i < count; i++) {
        if (entries[i].is_dir) {
            printf("%s%s/\n", prefix, entries[i].name);
            char np[1024];
            snprintf(np, sizeof(np), "%s%s/", prefix, entries[i].name);
            list_recursive(entries[i].child_chain, np, depth + 1);
        } else {
            printf("%s%s  (%u bytes)\n", prefix, entries[i].name, entries[i].size);
        }
    }
}

static void cmd_list(void) {
    LumSuperblock sb;
    if (read_sb(&sb) != 0 || sb.magic != LUM_MAGIC) {
        fprintf(stderr, "Error: not a valid LumFS partition\n"); return;
    }
    printf("/ (root) - %u files\n\n", sb.file_count);
    list_recursive(LUM_ROOT_CHAIN, "", 0);
}

static void cmd_cat(const char *path) {
    uint32_t chain = resolve_path(path);
    if (chain == 0) { fprintf(stderr, "Error: not found: %s\n", path); return; }
    LumEntryInfo info;
    if (get_chain_info(chain, &info) == 0 && info.is_dir) {
        fprintf(stderr, "Error: %s is a directory\n", path); return;
    }
    if (info.size == 0) return;
    uint8_t *buf = (uint8_t *)malloc(info.size + 1);
    if (!buf) { fprintf(stderr, "Error: OOM\n"); return; }
    int64_t got = read_file_content(chain, buf, info.size);
    if (got < 0) { fprintf(stderr, "Error: read failed\n"); free(buf); return; }
    fwrite(buf, 1, (size_t)got, stdout);
    free(buf);
}

static void cmd_extract(const char *path, const char *output) {
    uint32_t chain = resolve_path(path);
    if (chain == 0) { fprintf(stderr, "Error: not found: %s\n", path); return; }
    LumEntryInfo info;
    if (get_chain_info(chain, &info) == 0 && info.is_dir) {
        fprintf(stderr, "Error: %s is a directory\n", path); return;
    }
    uint8_t *buf = (uint8_t *)malloc(info.size > 0 ? info.size : 1);
    if (!buf) { fprintf(stderr, "Error: OOM\n"); return; }
    int64_t got = (info.size > 0) ? read_file_content(chain, buf, info.size) : 0;
    if (got < 0) { fprintf(stderr, "Error: read failed\n"); free(buf); return; }
    FILE *f = fopen(output, "wb");
    if (!f) { fprintf(stderr, "Error: cannot create %s\n", output); free(buf); return; }
    if (got > 0) fwrite(buf, 1, (size_t)got, f);
    fclose(f); free(buf);
    printf("Extracted %lld bytes -> %s\n", (long long)got, output);
}

static void extract_tree(uint32_t chain_id, const char *dest_dir) {
    LumDirEntry entries[512];
    int count = list_dir_entries(chain_id, entries, 512);
    for (int i = 0; i < count; i++) {
        char fp[MAX_PATH];
        snprintf(fp, sizeof(fp), "%s\\%s", dest_dir, entries[i].name);
        if (entries[i].is_dir) {
            CreateDirectoryA(fp, NULL);
            extract_tree(entries[i].child_chain, fp);
        } else {
            uint8_t *buf = (uint8_t *)malloc(entries[i].size > 0 ? entries[i].size : 1);
            if (!buf) continue;
            int64_t got = (entries[i].size > 0) ?
                read_file_content(entries[i].child_chain, buf, entries[i].size) : 0;
            if (got > 0) {
                FILE *f = fopen(fp, "wb");
                if (f) { fwrite(buf, 1, (size_t)got, f); fclose(f); }
                printf("  %s (%lld bytes)\n", fp, (long long)got);
            }
            free(buf);
        }
    }
}

static void cmd_extract_dir(const char *path, const char *dest_dir) {
    uint32_t chain = resolve_path(path);
    if (chain == 0) { fprintf(stderr, "Error: not found: %s\n", path); return; }
    CreateDirectoryA(dest_dir, NULL);
    printf("Extracting to %s...\n", dest_dir);
    extract_tree(chain, dest_dir);
    printf("Done.\n");
}

/* ================================================================== */
/* Mount: extract + subst (pure Win32)                                 */
/*                                                                     */
/* Creates %TEMP%\lumfs_mnt, extracts all files there, runs            */
/* "subst X: path" to map a drive letter. Explorer sees it as a disk. */
/* On Ctrl+C or Enter, runs "subst /d X:" and cleans up.              */
/* ================================================================== */

static int s_unmount = 0;

static BOOL WINAPI ctrl_handler(DWORD type) {
    if (type == CTRL_C_EVENT || type == CTRL_CLOSE_EVENT) {
        s_unmount = 1;
        return TRUE;
    }
    return FALSE;
}

static void cmd_mount(const char *drive_letter) {
    LumSuperblock sb;
    if (read_sb(&sb) != 0 || sb.magic != LUM_MAGIC) {
        fprintf(stderr, "Error: not a valid LumFS partition\n"); return;
    }

    char letter = drive_letter[0];
    if (letter >= 'a' && letter <= 'z') letter -= 32;

    char mp[4] = { letter, ':', '\\', 0 };

    char mount_dir[MAX_PATH];
    snprintf(mount_dir, sizeof(mount_dir), "%s\\lumfs_mnt_%c", getenv("TEMP") ? getenv("TEMP") : ".", letter);

    printf("LumFS: %llu cells, %u files\n",
           (unsigned long long)sb.total_cells, sb.file_count);

    /* Remove old mount dir if exists */
    {
        char cmd[MAX_PATH + 64];
        snprintf(cmd, sizeof(cmd), "rmdir /s /q \"%s\" 2>nul", mount_dir);
        system(cmd);
    }

    CreateDirectoryA(mount_dir, NULL);
    printf("Extracting files to %s ...\n", mount_dir);
    extract_tree(LUM_ROOT_CHAIN, mount_dir);
    printf("Extraction done.\n");

    /* Map drive letter via subst */
    char subst_cmd[MAX_PATH + 64];
    snprintf(subst_cmd, sizeof(subst_cmd), "subst %s \"%s\"", mp, mount_dir);
    printf("Running: %s\n", subst_cmd);
    system(subst_cmd);

    printf("\n=== Mounted on %s ===\n", mp);
    printf("Open Explorer and go to %s\n", mp);
    printf("Press Enter or Ctrl+C to unmount...\n");

    SetConsoleCtrlHandler(ctrl_handler, TRUE);
    s_unmount = 0;

    while (!s_unmount) {
        if (GetAsyncKeyState(VK_RETURN) & 0x0001) { s_unmount = 1; break; }
        Sleep(200);
    }

    /* Unmount */
    char unmount_cmd[64];
    snprintf(unmount_cmd, sizeof(unmount_cmd), "subst /d %s", mp);
    printf("\nRunning: %s\n", unmount_cmd);
    system(unmount_cmd);

    /* Cleanup temp dir */
    {
        char cmd[MAX_PATH + 64];
        snprintf(cmd, sizeof(cmd), "rmdir /s /q \"%s\" 2>nul", mount_dir);
        system(cmd);
    }

    printf("Unmounted %s\n", mp);
}

static void cmd_unmount(const char *drive_letter) {
    char letter = drive_letter[0];
    if (letter >= 'a' && letter <= 'z') letter -= 32;
    char mp[4] = { letter, ':', '\\', 0 };
    char cmd[64];
    snprintf(cmd, sizeof(cmd), "subst /d %s", mp);
    system(cmd);
    printf("Unmounted %s\n", mp);
}

/* ================================================================== */
/* Usage & main                                                       */
/* ================================================================== */

static void print_usage(const char *p) {
    printf(
        "LumFS Tool (pure Win32)\n\n"
        "Usage:\n"
        "  %s info       <device> [offset]\n"
        "  %s list       <device> [offset]\n"
        "  %s cat        <device> <path> [offset]\n"
        "  %s extract    <device> <path> <output> [offset]\n"
        "  %s allextract <device> <path> <dest_dir> [offset]\n"
        "  %s mount      <device> <letter:> [offset]\n"
        "  %s unmount    <letter:>\n"
        "\n"
        "mount = extract to temp + subst (no external deps)\n"
        "unmount = subst /d (removes drive mapping)\n"
        "\n"
        "Examples:\n"
        "  %s list \\\\.\\PhysicalDrive3\n"
        "  %s mount \\\\.\\PhysicalDrive3 Z:\n"
        "  %s unmount Z:\n"
        "\n"
        "Find USB: wmic diskdrive list brief\n",
        p,p,p,p,p,p,p,p,p,p);
}

int main(int argc, char *argv[]) {
    if (argc < 2) { print_usage(argv[0]); return 1; }

    const char *cmd = argv[1];

    if (strcmp(cmd, "unmount") == 0) {
        if (argc < 3) { fprintf(stderr, "Need drive letter\n"); return 1; }
        cmd_unmount(argv[2]);
        return 0;
    }

    if (argc < 3) { print_usage(argv[0]); return 1; }
    const char *device = argv[2];

    g_cell_buf = (uint8_t *)malloc(LUM_CELL_SIZE);
    if (!g_cell_buf) { fprintf(stderr, "OOM\n"); return 1; }

    g_offset = 0;
    if (strcmp(cmd, "list") == 0 && argc >= 4)       g_offset = _atoi64(argv[3]);
    else if (strcmp(cmd, "info") == 0 && argc >= 4)   g_offset = _atoi64(argv[3]);
    else if (strcmp(cmd, "cat") == 0 && argc >= 5)    g_offset = _atoi64(argv[4]);
    else if (strcmp(cmd, "extract") == 0 && argc >= 6)    g_offset = _atoi64(argv[5]);
    else if (strcmp(cmd, "allextract") == 0 && argc >= 6) g_offset = _atoi64(argv[5]);
    else if (strcmp(cmd, "mount") == 0 && argc >= 5)  g_offset = _atoi64(argv[4]);

    DWORD attr = GetFileAttributesA(device);
    if (attr != INVALID_FILE_ATTRIBUTES && !(attr & FILE_ATTRIBUTE_DIRECTORY)) {
        g_is_file = 1;
        g_disk = CreateFileA(device, GENERIC_READ, FILE_SHARE_READ|FILE_SHARE_WRITE,
                            NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    }
    if (g_disk == INVALID_HANDLE_VALUE) {
        g_is_file = 0;
        g_disk = CreateFileA(device, GENERIC_READ, FILE_SHARE_READ|FILE_SHARE_WRITE,
                            NULL, OPEN_EXISTING, 0, NULL);
    }
    if (g_disk == INVALID_HANDLE_VALUE) {
        fprintf(stderr, "Error: cannot open %s (err %lu)\n", device, GetLastError());
        fprintf(stderr, "Tip: run as Administrator for physical disk access\n");
        free(g_cell_buf); return 1;
    }

    if (g_offset == 0 && !g_is_file) {
        LumSuperblock sb_test;
        int found = 0;
        if (read_cell(0, g_cell_buf) == 0) {
            memcpy(&sb_test, g_cell_buf, sizeof(sb_test));
            if (sb_test.magic == LUM_MAGIC && sb_test.version == LUM_VERSION) found = 1;
        }
        if (!found) {
            uint64_t tries[] = { 2048, 63, 64, 4096, 8192, 40960 };
            for (int i = 0; i < 6; i++) {
                g_offset = tries[i];
                if (read_cell(0, g_cell_buf) == 0) {
                    memcpy(&sb_test, g_cell_buf, sizeof(sb_test));
                    if (sb_test.magic == LUM_MAGIC && sb_test.version == LUM_VERSION) {
                        fprintf(stderr, "Found LumFS at sector %llu\n",
                                (unsigned long long)g_offset);
                        found = 1; break;
                    }
                }
            }
        }
        if (!found) {
            fprintf(stderr, "Warning: LumFS not found, trying offset 0\n");
            g_offset = 0;
        }
    }

    if      (strcmp(cmd, "info") == 0)       cmd_info();
    else if (strcmp(cmd, "list") == 0)       cmd_list();
    else if (strcmp(cmd, "cat") == 0)        { if (argc<4) { fprintf(stderr,"Need path\n"); return 1; } cmd_cat(argv[3]); }
    else if (strcmp(cmd, "extract") == 0)    { if (argc<5) { fprintf(stderr,"Need path+output\n"); return 1; } cmd_extract(argv[3], argv[4]); }
    else if (strcmp(cmd, "allextract") == 0) { if (argc<5) { fprintf(stderr,"Need path+dest\n"); return 1; } cmd_extract_dir(argv[3], argv[4]); }
    else if (strcmp(cmd, "mount") == 0)      { if (argc<4) { fprintf(stderr,"Need letter\n"); return 1; } cmd_mount(argv[3]); }
    else { fprintf(stderr, "Unknown: %s\n", cmd); print_usage(argv[0]); }

    CloseHandle(g_disk);
    free(g_cell_buf);
    return 0;
}
