#!/usr/bin/env python3
"""Build a format-2 versionlib .bin from the Skyrim VR Address Library CSV.

The VR Address Library ships a CSV keyed by *SE* ids, while this project looks
addresses up by *AE* ids in a meh321 format-2 .bin. This script converts the CSV
and injects the extra `id=offset` pairs the plugin needs (see versionlib/VR.md).

Usage:
    python tools/csv_to_versionlib_bin.py <version-1-4-15-0.csv> <out.bin> [id=hexoffset ...]
"""
import struct
import sys

VERSION = (1, 4, 15, 0)
MODULE_NAME = b"SkyrimVR.exe"
POINTER_SIZE = 8


def read_csv(path):
    """Line 1 is the header, line 2 is `<count>,<library version>`, then `<id>,<hex offset>`."""
    entries = {}
    with open(path, "r", encoding="ascii") as f:
        for line in f.read().splitlines()[2:]:
            if not line.strip():
                continue
            address_id, offset = line.split(",")[:2]
            entries[int(address_id)] = int(offset, 16)
    return entries


def write_bin(path, entries):
    with open(path, "wb") as f:
        f.write(struct.pack("<i4i", 2, *VERSION))
        f.write(struct.pack("<i", len(MODULE_NAME)) + MODULE_NAME)
        f.write(struct.pack("<ii", POINTER_SIZE, len(entries)))
        for address_id, offset in sorted(entries.items()):
            # type byte 0x00: id and offset both stored as absolute u64 (no delta encoding)
            f.write(struct.pack("<BQQ", 0, address_id, offset))


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 2
    entries = read_csv(argv[1])
    for pair in argv[3:]:
        address_id, offset = pair.split("=")
        entries[int(address_id)] = int(offset, 16)
    write_bin(argv[2], entries)
    print(f"wrote {argv[2]}: {len(entries)} entries")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
