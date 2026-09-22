#!/usr/bin/env python3
"""Report the EABI build attributes of an ELF object or static archive.

Answers the questions that decide whether a Rust static library can be linked
into an FSP project at all, and whether its bindings describe the same structs
the C side sees:

  Tag_ABI_enum_size  small (1) means -fshort-enums; every enum-bearing struct
                     changes shape between the two settings, with no
                     diagnostic from either compiler. DESIGN.md 2.3.
  Tag_ABI_VFP_args   hard (1) means float arguments travel in VFP registers.
                     A mismatch here is silent and corrupts every
                     float-passing call. DESIGN.md 2.5.
  Tag_FP_arch        which FPU the code was built for.

Usage:
    fsp-sys/script/check-arm-abi.py <file.a | file.o | file.elf> ...

Reads ELF and `ar` structure directly so it works without binutils, which no
Renesas toolchain installs on a developer's PATH under its own name.
"""

import struct
import sys

# Selected AEABI attribute tags (Arm IHI 0045, "Addenda to ABI for the Arm
# Architecture"). Only the ones this check reports are listed.
TAGS = {
    6: ("Tag_CPU_arch", {
        0: "Pre-v4", 1: "v4", 2: "v4T", 3: "v5T", 4: "v5TE", 5: "v5TEJ",
        6: "v6", 7: "v6KZ", 8: "v6T2", 9: "v6K", 10: "v7", 11: "v6-M",
        12: "v6S-M", 13: "v7E-M", 14: "v8-A", 15: "v8-R", 16: "v8-M.baseline",
        17: "v8-M.mainline", 21: "v8.1-M.mainline",
    }),
    7: ("Tag_CPU_arch_profile", {0x41: "A", 0x52: "R", 0x4D: "M", 0x53: "S"}),
    10: ("Tag_FP_arch", {
        0: "none", 1: "VFPv1", 2: "VFPv2", 3: "VFPv3", 4: "VFPv3-D16",
        5: "VFPv4", 6: "VFPv4-D16", 7: "FPv5/FP-ARMv8-D16", 8: "FP-ARMv8",
    }),
    18: ("Tag_ABI_PCS_wchar_t", {}),
    20: ("Tag_ABI_FP_denormal", {}),
    26: ("Tag_ABI_enum_size", {
        0: "unused", 1: "small (-fshort-enums)", 2: "int (32-bit)",
        3: "either (visible externally as int)",
    }),
    28: ("Tag_ABI_VFP_args", {
        0: "base (soft-float ABI)", 1: "VFP (hard-float ABI)",
        2: "toolchain-specific", 3: "compatible with both",
    }),
    34: ("Tag_CPU_unaligned_access", {0: "none", 1: "v6-style"}),
}

# Tags whose value is a NUL-terminated string rather than a ULEB128.
STRING_TAGS = {4, 5, 32, 65, 67}


def uleb(buf, i):
    result = shift = 0
    while True:
        byte = buf[i]
        i += 1
        result |= (byte & 0x7F) << shift
        if not byte & 0x80:
            return result, i
        shift += 7


def cstr(buf, i):
    end = buf.index(b"\0", i)
    return buf[i:end].decode("utf-8", "replace"), end + 1


def elf_section(data, name):
    """Return the bytes of a named section in a 32-bit little-endian ELF."""
    if data[:4] != b"\x7fELF" or data[4] != 1 or data[5] != 1:
        return None
    e_shoff, = struct.unpack_from("<I", data, 0x20)
    e_shentsize, e_shnum, e_shstrndx = struct.unpack_from("<HHH", data, 0x2E)
    if not e_shoff:
        return None

    def header(n):
        return struct.unpack_from("<10I", data, e_shoff + n * e_shentsize)

    strtab_off = header(e_shstrndx)[4]
    for n in range(e_shnum):
        sh_name, _, _, _, sh_offset, sh_size = header(n)[:6]
        if cstr(data, strtab_off + sh_name)[0] == name:
            return data[sh_offset:sh_offset + sh_size]
    return None


def parse_attributes(sec):
    """Decode the aeabi subsection of a .ARM.attributes section."""
    if not sec or sec[0] != ord("A"):
        return {}
    pos = 1
    while pos < len(sec):
        length, = struct.unpack_from("<I", sec, pos)
        vendor, i = cstr(sec, pos + 4)
        end = pos + length
        pos = end
        if vendor != "aeabi":
            continue
        while i < end:
            # A sub-subsection's length counts its own tag and length fields.
            start = i
            tag, i = uleb(sec, i)
            size, = struct.unpack_from("<I", sec, i)
            sub_end = start + size
            i += 4
            if tag != 1:  # 1 == File scope; section/symbol scope is unused here
                i = sub_end
                continue
            attrs = {}
            while i < sub_end:
                attr, i = uleb(sec, i)
                if attr in STRING_TAGS:
                    value, i = cstr(sec, i)
                else:
                    value, i = uleb(sec, i)
                attrs[attr] = value
            return attrs
    return {}


def objects(path):
    """Yield (label, bytes) for an ELF file, or for each member of an archive."""
    with open(path, "rb") as fh:
        data = fh.read()
    if data[:8] != b"!<arch>\n":
        yield path, data
        return
    pos, names = 8, b""
    while pos + 60 <= len(data):
        name = data[pos:pos + 16].rstrip().decode("ascii", "replace")
        size = int(data[pos + 48:pos + 58].strip() or 0)
        body = data[pos + 60:pos + 60 + size]
        pos += 60 + size + (size & 1)
        if name == "//":
            names = body
        elif name.startswith("/") and name[1:].isdigit():
            off = int(name[1:])
            yield names[off:names.index(b"/", off)].decode(), body
        elif name not in ("/", "/SYM64/", "__.SYMDEF", "__.SYMDEF SORTED"):
            yield name.rstrip("/"), body


def main(argv):
    if len(argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    for path in argv[1:]:
        print(path)
        seen = set()
        for label, body in objects(path):
            attrs = parse_attributes(elf_section(body, ".ARM.attributes"))
            if not attrs:
                continue
            key = tuple(sorted(attrs.items()))
            if key in seen:
                continue
            seen.add(key)
            print(f"  {label}")
            for tag, value in sorted(attrs.items()):
                if tag not in TAGS:
                    continue
                name, meanings = TAGS[tag]
                print(f"    {name:24} {meanings.get(value, value)}")
        if not seen:
            print("  no aeabi build attributes found")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
