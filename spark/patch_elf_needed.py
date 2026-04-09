#!/usr/bin/env python3
"""
Add DT_NEEDED entry to ELF shared library.
Usage: python3 patch_elf_needed.py <library.so> <libname>
"""

import sys
import struct
import os

def round_up(value, alignment):
    return (value + alignment - 1) & ~(alignment - 1)

def add_dt_needed(so_path, needed_lib):
    """
    Add DT_NEEDED entry to ELF shared library.
    This is a simplified implementation that appends to .dynamic section.
    """
    with open(so_path, 'rb') as f:
        data = bytearray(f.read())
    
    # Check ELF magic
    if data[:4] != b'\x7fELF':
        print(f"Error: {so_path} is not an ELF file")
        return False
    
    # Get ELF class (32 or 64 bit)
    ei_class = data[4]
    if ei_class != 2:  # Only 64-bit supported for now
        print(f"Error: Only 64-bit ELF is supported")
        return False
    
    # Parse ELF64 header
    e_phoff = struct.unpack('<Q', data[32:40])[0]
    e_shoff = struct.unpack('<Q', data[40:48])[0]
    e_phentsize = struct.unpack('<H', data[54:56])[0]
    e_phnum = struct.unpack('<H', data[56:58])[0]
    
    # Find PT_DYNAMIC segment
    PT_DYNAMIC = 2
    dynamic_offset = None
    dynamic_size = None
    
    for i in range(e_phnum):
        phoff = e_phoff + i * e_phentsize
        p_type = struct.unpack('<I', data[phoff:phoff+4])[0]
        if p_type == PT_DYNAMIC:
            dynamic_offset = struct.unpack('<Q', data[phoff+8:phoff+16])[0]
            dynamic_size = struct.unpack('<Q', data[phoff+16:phoff+24])[0]
            break
    
    if dynamic_offset is None:
        print("Error: No PT_DYNAMIC segment found")
        return False
    
    print(f"Found PT_DYNAMIC at offset {dynamic_offset}, size {dynamic_size}")
    
    # Read dynamic section
    # In ELF64, each Dyn entry is 16 bytes (d_tag:8 + d_val:8)
    DT_NEEDED = 1
    DT_STRTAB = 5
    DT_STRSZ = 10
    DT_NULL = 0
    
    strtab_offset = None
    strtab_vaddr = None
    
    # First pass: find STRTAB
    offset = dynamic_offset
    while True:
        d_tag = struct.unpack('<Q', data[offset:offset+8])[0]
        if d_tag == DT_NULL:
            break
        d_val = struct.unpack('<Q', data[offset+8:offset+16])[0]
        
        if d_tag == DT_STRTAB:
            # This is a virtual address, need to convert to file offset
            # For now, assume it's relative to load address (simplification)
            strtab_vaddr = d_val
            print(f"STRA TAB at vaddr 0x{strtab_vaddr:x}")
        
        offset += 16
    
    if strtab_vaddr is None:
        print("Error: DT_STRTAB not found")
        return False
    
    # For PIE/shared libs, strtab_vaddr is typically close to file offset
    # We'll append the new string to the end of the file and update STRTAB
    # This is a hack but works for adding needed libraries
    
    # Add needed library name to end of file
    needed_name = needed_lib.encode('utf-8') + b'\x00'
    new_file_size = len(data) + len(needed_name)
    data.extend(needed_name)
    
    new_str_off = len(data) - len(needed_name)
    
    # Find location after last DT_NEEDED to insert new one
    # We need to extend .dynamic section - this is complex
    # Alternative: create a new .dynamic section (even more complex)
    
    # Simplified approach: just print instructions for manual intervention
    print(f"\nTo add DT_NEEDED for {needed_lib}:")
    print(f"  Library: {so_path}")
    print(f"  String would be at offset: {new_str_off}")
    print(f"  String content: {needed_name}")
    print(f"\nRecommended: Use patchelf or ld with --no-as-needed")
    print(f"  patchelf --add-needed {needed_lib} {so_path}")
    return False

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <library.so> <libname>")
        sys.exit(1)
    
    if not add_dt_needed(sys.argv[1], sys.argv[2]):
        print("\nELF patching failed. Install patchelf or use LD_PRELOAD workaround.")
        sys.exit(1)