#!/usr/bin/env python3
"""Validate a built .endl against the SDK's PatchHeader layout (internal/PatchABI.h)."""
import struct, sys, pathlib

MAGIC, ABI = 0x48435450, 0x000B
LOAD, RAM_LEN = 0x80000000, 512 * 1024
FN_NAMES = ["init", "agent_update_buffers", "agent_set_buffer", "agent_get_buffer_size",
            "agent_get_param_min", "agent_get_param_max", "agent_get_param_default",
            "agent_is_param_enabled", "agent_get_param_name", "agent_get_param_unit",
            "agent_set_param", "agent_special_action", "agent_get_state_idx"]

def main(path):
    data = pathlib.Path(path).read_bytes()
    fails = 0
    def check(ok, msg):
        nonlocal fails
        print(("  ok   " if ok else "  FAIL ") + msg)
        fails += not ok

    hdr = "<IHH" + "I" * len(FN_NAMES) + "III" + "16I"
    size = struct.calcsize(hdr)
    check(len(data) >= size, f"file holds a full header ({len(data)} bytes)")
    f = struct.unpack_from(hdr, data, 0)
    magic, abi, flags = f[0], f[1], f[2]
    fns = f[3:3 + len(FN_NAMES)]
    image_size, bss_begin, bss_size = f[3 + len(FN_NAMES):6 + len(FN_NAMES)]
    reserved = f[6 + len(FN_NAMES):]

    check(magic == MAGIC, f"magic 'PTCH' (0x{magic:08x})")
    check(abi == ABI, f"ABI version 0x{abi:04x} == 0x{ABI:04x}")
    check(flags == 0, f"flags == 0 ({flags})")
    for name, addr in zip(FN_NAMES, fns):
        in_ram = LOAD <= addr < LOAD + len(data)
        check(in_ram and (addr & 1), f"{name} → 0x{addr:08x} inside image, Thumb bit set")
    # Per patch_imx.ld, image_size excludes the header: the file is header + alignment padding + image, and bss follows the file in RAM. The padding between the
    # header and __patch_start depends on the largest section alignment in the build (0 bytes at -O3, 16 bytes at -O0), so allow a small gap but nothing more.
    gap = len(data) - (size + image_size)
    check(0 <= gap < 64, f"header ({size}) + image_size ({image_size}) == file size {len(data)} up to alignment padding (gap {gap})")
    check(bss_begin == LOAD + len(data), f"bss starts right after the file image (0x{bss_begin:08x})")
    check(len(data) + bss_size <= RAM_LEN, f"file + bss = {len(data) + bss_size} bytes ≤ {RAM_LEN}")
    # Advisory only: a large .bss is fine as long as everything fits (the line above).
    print(("  ok   " if bss_size < 64 * 1024 else "  note ") + f"bss is {bss_size} bytes" + ("" if bss_size < 64 * 1024 else " (large; fine while file + bss fit)"))
    check(all(v == 0 for v in reserved), "reserved words are zero")
    print(f"{'PASS' if not fails else 'FAIL'}: {path}")
    return fails

if __name__ == "__main__":
    sys.exit(1 if main(sys.argv[1]) else 0)
