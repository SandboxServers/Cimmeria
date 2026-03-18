"""
SGW UE3 Package Parser - Proof of Concept
==========================================
Parses .upk and .umap files from Stargate Worlds (Epic Version 486, Licensee 6-8).

Format reverse-engineered from:
- UE3 source reference (Core/Inc/UnLinker.h, Core/Src/UnLinker.cpp)
- Ghidra decompilation of SGW.exe (FUN_004bcab0, FUN_004bc9b0, etc.)

Binary format details:
- FPackageFileSummary: Variable-length header with version-conditional fields
- FNameEntry: FString (i32 len + ASCII) + u64 flags = variable length
- FObjectImport: ClassPackage(FName) + ClassName(FName) + PackageIndex(i32) + ObjectName(FName) = 28 bytes fixed
- FObjectExport: Variable length due to ComponentMap (TMap<FName,i32>) and GenNetObjCount (TArray<i32>)
"""

import struct
import sys
import os
import zlib
import io
from dataclasses import dataclass, field
from typing import List, Optional, Tuple, BinaryIO

try:
    import lzallright
    _lzo = lzallright.LZOCompressor()
    def _lzo_decompress(data: bytes) -> bytes:
        return _lzo.decompress(data)
    HAS_LZO = True
except ImportError:
    HAS_LZO = False
    def _lzo_decompress(data: bytes) -> bytes:
        raise RuntimeError("LZO decompression requires 'lzallright' package: pip install lzallright")

PACKAGE_FILE_TAG = 0x9E2A83C1

# UE3 compression flags (early UE3 enum values used by SGW)
COMPRESS_ZLIB = 0x01
COMPRESS_LZO = 0x02
COMPRESS_LZX = 0x04


@dataclass
class PackageHeader:
    tag: int = 0
    epic_version: int = 0
    licensee_version: int = 0
    total_header_size: int = 0
    folder_name: str = ""
    package_flags: int = 0
    name_count: int = 0
    name_offset: int = 0
    export_count: int = 0
    export_offset: int = 0
    import_count: int = 0
    import_offset: int = 0
    depends_offset: int = 0
    guid: Tuple[int, int, int, int] = (0, 0, 0, 0)
    generations: list = field(default_factory=list)
    engine_version: int = 0
    cooker_version: int = 0
    compression_flags: int = 0
    compressed_chunks: list = field(default_factory=list)


@dataclass
class NameEntry:
    name: str = ""
    flags: int = 0


@dataclass
class ImportEntry:
    class_package: str = ""  # Resolved from name table
    class_name: str = ""     # Resolved from name table
    package_index: int = 0   # Object index (negative = import, positive = export)
    object_name: str = ""    # Resolved from name table
    # Raw FName indices for debugging
    class_package_idx: int = 0
    class_name_idx: int = 0
    object_name_idx: int = 0


@dataclass
class ExportEntry:
    class_index: int = 0
    super_index: int = 0
    package_index: int = 0     # Outer
    object_name: str = ""      # Resolved from name table
    object_name_idx: int = 0
    object_name_num: int = 0
    archetype: int = 0
    object_flags: int = 0      # u64
    serial_size: int = 0
    serial_offset: int = 0
    component_map: list = field(default_factory=list)  # List of (FName_str, i32)
    export_flags: int = 0
    gen_net_obj_count: list = field(default_factory=list)
    package_guid: Tuple[int, int, int, int] = (0, 0, 0, 0)


class PackageReader:
    """Reads UE3 package files (SGW variant, Epic version 486)."""

    def __init__(self, filepath: str):
        self.filepath = filepath
        self.f: Optional[BinaryIO] = None
        self._decompressed: Optional[BinaryIO] = None
        self.header = PackageHeader()
        self.names: List[NameEntry] = []
        self.imports: List[ImportEntry] = []
        self.exports: List[ExportEntry] = []

    def read_i32(self) -> int:
        return struct.unpack('<i', self.f.read(4))[0]

    def read_u32(self) -> int:
        return struct.unpack('<I', self.f.read(4))[0]

    def read_u64(self) -> int:
        return struct.unpack('<Q', self.f.read(8))[0]

    def read_i64(self) -> int:
        return struct.unpack('<q', self.f.read(8))[0]

    def read_fstring(self) -> str:
        """Read a length-prefixed string (UE3 FString serialization).
        Length is i32; positive = ASCII, negative = UTF-16LE (abs(len) chars)."""
        length = self.read_i32()
        if length == 0:
            return ""
        if length < 0:
            # UTF-16LE string
            char_count = -length
            raw = self.f.read(char_count * 2)
            return raw.decode('utf-16-le').rstrip('\x00')
        else:
            # ASCII string (length includes null terminator)
            raw = self.f.read(length)
            return raw.rstrip(b'\x00').decode('ascii', errors='replace')

    def read_fname(self) -> Tuple[int, int]:
        """Read an FName from the archive: (name_index, instance_number)."""
        name_index = self.read_i32()
        instance_number = self.read_i32()
        return (name_index, instance_number)

    def resolve_name(self, index: int) -> str:
        """Resolve a name table index to a string."""
        if 0 <= index < len(self.names):
            return self.names[index].name
        return f"<invalid_name_{index}>"

    def resolve_object_name(self, obj_index: int) -> str:
        """Resolve a package object index to a readable name.
        Positive = export (1-based), negative = import (negated 1-based), 0 = None."""
        if obj_index == 0:
            return "None"
        elif obj_index < 0:
            idx = -obj_index - 1
            if 0 <= idx < len(self.imports):
                return self.imports[idx].object_name
            return f"<import_{idx}>"
        else:
            idx = obj_index - 1
            if 0 <= idx < len(self.exports):
                return self.exports[idx].object_name
            return f"<export_{idx}>"

    def parse_header(self):
        """Parse FPackageFileSummary (FUN_004bcab0 in SGW.exe)."""
        self.f.seek(0)
        h = self.header

        h.tag = self.read_u32()
        if h.tag != PACKAGE_FILE_TAG:
            raise ValueError(f"Invalid package tag: 0x{h.tag:08X} (expected 0x{PACKAGE_FILE_TAG:08X})")

        packed_version = self.read_u32()
        h.epic_version = packed_version & 0xFFFF
        h.licensee_version = (packed_version >> 16) & 0xFFFF

        # TotalHeaderSize (if Epic >= 249)
        if h.epic_version >= 249:
            h.total_header_size = self.read_i32()

        # FolderName (if Epic >= 269)
        if h.epic_version >= 269:
            h.folder_name = self.read_fstring()

        h.package_flags = self.read_u32()
        h.name_count = self.read_i32()
        h.name_offset = self.read_i32()
        h.export_count = self.read_i32()
        h.export_offset = self.read_i32()
        h.import_count = self.read_i32()
        h.import_offset = self.read_i32()

        # DependsOffset (if Epic > 414)
        if h.epic_version > 414:
            h.depends_offset = self.read_i32()

        # GUID
        h.guid = (self.read_u32(), self.read_u32(), self.read_u32(), self.read_u32())

        # Generations
        gen_count = self.read_i32()
        h.generations = []
        for _ in range(gen_count):
            exp_count = self.read_i32()
            name_count = self.read_i32()
            net_obj_count = self.read_i32()
            h.generations.append((exp_count, name_count, net_obj_count))

        # EngineVersion (if Epic >= 245)
        if h.epic_version >= 245:
            h.engine_version = self.read_i32()

        # CookerVersion (if Epic >= 277)
        if h.epic_version >= 277:
            h.cooker_version = self.read_i32()

        # Compression (if Epic > 333)
        if h.epic_version > 333:
            h.compression_flags = self.read_u32()
            chunk_count = self.read_i32()
            h.compressed_chunks = []
            for _ in range(chunk_count):
                uncomp_offset = self.read_i32()
                uncomp_size = self.read_i32()
                comp_offset = self.read_i32()
                comp_size = self.read_i32()
                h.compressed_chunks.append((uncomp_offset, uncomp_size, comp_offset, comp_size))

    def parse_names(self):
        """Parse the name table. Each entry: FString + u64 flags."""
        self.f.seek(self.header.name_offset)
        self.names = []
        for _ in range(self.header.name_count):
            name = self.read_fstring()
            flags = self.read_u64()
            self.names.append(NameEntry(name=name, flags=flags))

    def parse_imports(self):
        """Parse the import table. Each entry: 3×FName + 1×i32 = 28 bytes fixed.
        Disk order: ClassPackage(FName), ClassName(FName), PackageIndex(i32), ObjectName(FName)."""
        self.f.seek(self.header.import_offset)
        self.imports = []
        for _ in range(self.header.import_count):
            cp_idx, cp_num = self.read_fname()
            cn_idx, cn_num = self.read_fname()
            pkg_index = self.read_i32()
            on_idx, on_num = self.read_fname()

            entry = ImportEntry(
                class_package=self.resolve_name(cp_idx),
                class_name=self.resolve_name(cn_idx),
                package_index=pkg_index,
                object_name=self.resolve_name(on_idx),
                class_package_idx=cp_idx,
                class_name_idx=cn_idx,
                object_name_idx=on_idx,
            )
            self.imports.append(entry)

    def parse_exports(self):
        """Parse the export table. Variable-length records due to ComponentMap and GenNetObjCount.
        Reverse-engineered from FUN_004bc9b0 in SGW.exe via Ghidra.

        Disk order (for SGW Epic version 486):
        1. ClassIndex: i32
        2. SuperIndex: i32
        3. PackageIndex: i32 (OuterIndex)
        4. ObjectName: FName (i32 + i32 = 8 bytes)
        5. Archetype: i32
        6. ObjectFlags: u64 (8 bytes)
        7. SerialSize: i32
        8. SerialOffset: i32 (always present, version > 248)
        9. ComponentMap: TMap<FName, i32> = i32 count + count × (FName{8} + i32{4})
        10. ExportFlags: u32 (always present, version >= 247)
        11. GenerationNetObjectCount: TArray<i32> = i32 count + count × i32
        12. PackageGuid: FGuid = 4 × i32 (version > 321)
        """
        self.f.seek(self.header.export_offset)
        self.exports = []
        for _ in range(self.header.export_count):
            e = ExportEntry()

            e.class_index = self.read_i32()
            e.super_index = self.read_i32()
            e.package_index = self.read_i32()

            name_idx, name_num = self.read_fname()
            e.object_name_idx = name_idx
            e.object_name_num = name_num
            e.object_name = self.resolve_name(name_idx)

            e.archetype = self.read_i32()
            e.object_flags = self.read_u64()
            e.serial_size = self.read_i32()

            # SerialOffset: always present for version > 248 (SGW = 486)
            if e.serial_size > 0 or self.header.epic_version > 248:
                e.serial_offset = self.read_i32()

            # ComponentMap: TMap<FName, i32>
            comp_count = self.read_i32()
            e.component_map = []
            for _ in range(comp_count):
                cm_name_idx, cm_name_num = self.read_fname()
                cm_value = self.read_i32()
                cm_name = self.resolve_name(cm_name_idx)
                e.component_map.append((cm_name, cm_value))

            # ExportFlags (version >= 247)
            if self.header.epic_version >= 247:
                e.export_flags = self.read_u32()

            # GenerationNetObjectCount (version > 321)
            if self.header.epic_version > 321:
                gen_count = self.read_i32()
                e.gen_net_obj_count = []
                for _ in range(gen_count):
                    e.gen_net_obj_count.append(self.read_i32())

                # PackageGuid: 4 × i32
                e.package_guid = (self.read_u32(), self.read_u32(), self.read_u32(), self.read_u32())

            self.exports.append(e)

    def _decompress_package(self, raw_f: BinaryIO) -> BinaryIO:
        """Decompress a compressed package into a memory stream.
        UE3 compressed packages use chunked compression where each chunk
        contains sub-blocks of max 128KB (0x20000) compressed data."""
        h = self.header
        if h.compression_flags == 0 or len(h.compressed_chunks) == 0:
            return raw_f

        # Read the header portion (before compressed data starts)
        # The header up to the first compressed chunk offset is uncompressed
        header_end = h.compressed_chunks[0][2]  # comp_offset of first chunk
        raw_f.seek(0)
        header_data = raw_f.read(header_end)

        # Calculate total uncompressed size
        last_chunk = h.compressed_chunks[-1]
        total_uncomp_size = last_chunk[0] + last_chunk[1]  # offset + size of last chunk

        # Build the full uncompressed buffer
        uncomp_data = bytearray(total_uncomp_size)
        # Copy header as-is (the offsets in the summary treat offset 0 = file start)
        uncomp_data[0:len(header_data)] = header_data

        for uncomp_offset, uncomp_size, comp_offset, comp_size in h.compressed_chunks:
            raw_f.seek(comp_offset)
            chunk_data = raw_f.read(comp_size)

            # Each chunk has a sub-block header:
            # u32 tag (PACKAGE_FILE_TAG), u32 block_size (max uncompressed per sub-block),
            # u32 total_comp_size, u32 total_uncomp_size,
            # then N sub-blocks, each: u32 comp_size, u32 uncomp_size
            pos = 0
            tag = struct.unpack_from('<I', chunk_data, pos)[0]
            pos += 4
            block_size = struct.unpack_from('<I', chunk_data, pos)[0]  # typically 0x20000 (128KB)
            pos += 4
            total_comp = struct.unpack_from('<i', chunk_data, pos)[0]
            pos += 4
            total_uncomp = struct.unpack_from('<i', chunk_data, pos)[0]
            pos += 4

            # Calculate number of sub-blocks
            num_blocks = (total_uncomp + block_size - 1) // block_size

            # Read sub-block sizes
            sub_blocks = []
            for _ in range(num_blocks):
                sb_comp = struct.unpack_from('<i', chunk_data, pos)[0]
                pos += 4
                sb_uncomp = struct.unpack_from('<i', chunk_data, pos)[0]
                pos += 4
                sub_blocks.append((sb_comp, sb_uncomp))

            # Decompress sub-blocks
            write_pos = uncomp_offset
            for sb_comp, sb_uncomp in sub_blocks:
                compressed_block = chunk_data[pos:pos + sb_comp]
                pos += sb_comp

                if h.compression_flags & COMPRESS_LZO:
                    decompressed = _lzo_decompress(compressed_block)
                elif h.compression_flags & COMPRESS_ZLIB:
                    try:
                        decompressed = zlib.decompress(compressed_block)
                    except zlib.error:
                        decompressed = zlib.decompress(compressed_block, -zlib.MAX_WBITS)
                else:
                    raise ValueError(f"Unsupported compression: 0x{h.compression_flags:X}")

                uncomp_data[write_pos:write_pos + len(decompressed)] = decompressed
                write_pos += len(decompressed)

        return io.BytesIO(bytes(uncomp_data))

    def parse(self):
        """Parse the complete package."""
        with open(self.filepath, 'rb') as f:
            self.f = f
            self.parse_header()

            # If compressed, decompress and re-parse from memory
            if self.header.compression_flags != 0 and len(self.header.compressed_chunks) > 0:
                self.f = self._decompress_package(f)

            self.parse_names()
            self.parse_imports()
            self.parse_exports()
            self._decompressed = self.f  # Keep reference for serial data reading
            self.f = None

    def _get_data_stream(self) -> BinaryIO:
        """Get a readable stream for serial data (reopens file or uses cached decompressed)."""
        if self._decompressed is not None and isinstance(self._decompressed, io.BytesIO):
            return self._decompressed
        return open(self.filepath, 'rb')

    def _find_name_index(self, name: str) -> int:
        """Find the index of a name in the name table."""
        for i, n in enumerate(self.names):
            if n.name == name:
                return i
        return -1

    def _is_actor_class(self, class_name: str) -> bool:
        """Check if a class name is an AActor subclass (has 32-byte header)."""
        # Known actor classes from SGW .umap files
        actor_classes = {
            'StaticMeshActor', 'Brush', 'Terrain', 'WorldInfo',
            'SGWSpecCoverNode', 'PointLight', 'SpotLight', 'DirectionalLight',
            'AmbientSoundSGW', 'AmbientSound', 'SpeedTreeActor',
            'Trigger', 'TriggerVolume', 'BlockingVolume', 'PhysicsVolume',
            'PrefabInstance', 'DecalActor', 'Emitter', 'PointLightMovable',
            'Note', 'CameraActor', 'PlayerStart', 'PathNode',
            'InterpActor', 'SkeletalMeshActor', 'KActor',
            # SGW-specific actor types
            'SGWSpawnRegion', 'SGWSpawnSet', 'SGWCoverSet',
            'SGWMob', 'SGWNpc', 'SGWStargate', 'SGWInteractable',
            'SGWDoor', 'SGWTeleporter', 'SGWMinigame',
        }
        return class_name in actor_classes

    def _is_component_class(self, class_name: str) -> bool:
        """Check if a class is a component (has 8-byte header)."""
        return 'Component' in class_name

    def parse_tagged_properties(self, data: bytes, offset: int = 0) -> dict:
        """Parse UE3 tagged properties from serial data.

        Returns a dict of property_name -> value (parsed where possible).
        """
        props = {}
        pos = offset

        while pos < len(data) - 8:
            # Read property name FName
            name_idx = struct.unpack_from('<i', data, pos)[0]
            name_num = struct.unpack_from('<i', data, pos + 4)[0]
            pos += 8

            if not (0 <= name_idx < len(self.names)):
                break

            name = self.names[name_idx].name
            if name == 'None':
                break

            # Read type FName
            type_idx = struct.unpack_from('<i', data, pos)[0]
            pos += 8
            type_name = self.names[type_idx].name if 0 <= type_idx < len(self.names) else ''

            # Read size and array index
            prop_size = struct.unpack_from('<i', data, pos)[0]
            pos += 4
            array_idx = struct.unpack_from('<i', data, pos)[0]
            pos += 4

            # Handle type-specific extra tags
            struct_type = None
            if type_name == 'StructProperty':
                si = struct.unpack_from('<i', data, pos)[0]
                pos += 8
                struct_type = self.names[si].name if 0 <= si < len(self.names) else None

            if type_name == 'BoolProperty':
                # BoolProperty: value stored in tag as i32, no value data
                bool_val = struct.unpack_from('<i', data, pos)[0]
                pos += 4
                key = f"{name}[{array_idx}]" if array_idx > 0 else name
                props[key] = bool(bool_val)
                continue

            if type_name == 'ByteProperty' and self.header.epic_version >= 445:
                # ByteProperty with enum: extra FName for enum type
                pos += 8

            # Read value
            if pos + prop_size > len(data):
                break

            value_data = data[pos:pos + prop_size]
            pos += prop_size

            key = f"{name}[{array_idx}]" if array_idx > 0 else name

            # Parse known types
            if type_name == 'FloatProperty' and prop_size == 4:
                props[key] = struct.unpack_from('<f', value_data, 0)[0]
            elif type_name == 'IntProperty' and prop_size == 4:
                props[key] = struct.unpack_from('<i', value_data, 0)[0]
            elif type_name == 'ObjectProperty' and prop_size == 4:
                props[key] = struct.unpack_from('<i', value_data, 0)[0]
            elif type_name == 'NameProperty' and prop_size == 8:
                ni = struct.unpack_from('<i', value_data, 0)[0]
                props[key] = self.names[ni].name if 0 <= ni < len(self.names) else f'?{ni}'
            elif type_name == 'StrProperty':
                props[key] = self._parse_fstring_from_bytes(value_data)
            elif type_name == 'StructProperty' and struct_type == 'Vector' and prop_size == 12:
                x, y, z = struct.unpack_from('<fff', value_data, 0)
                props[key] = (x, y, z)
            elif type_name == 'StructProperty' and struct_type == 'Rotator' and prop_size == 12:
                p, y2, r = struct.unpack_from('<iii', value_data, 0)
                props[key] = (p, y2, r)  # Raw UnrealRotator units
            elif type_name == 'StructProperty' and struct_type == 'Color' and prop_size == 4:
                b, g, r, a = struct.unpack_from('<BBBB', value_data, 0)
                props[key] = (r, g, b, a)
            elif type_name == 'StructProperty' and struct_type == 'LinearColor' and prop_size == 16:
                r, g, b, a = struct.unpack_from('<ffff', value_data, 0)
                props[key] = (r, g, b, a)
            else:
                props[key] = value_data  # Raw bytes for unknown types

        return props

    def _parse_fstring_from_bytes(self, data: bytes) -> str:
        """Parse an FString from raw bytes."""
        if len(data) < 4:
            return ''
        length = struct.unpack_from('<i', data, 0)[0]
        if length == 0:
            return ''
        if length > 0:
            return data[4:4 + length].rstrip(b'\x00').decode('ascii', errors='replace')
        else:
            count = -length
            return data[4:4 + count * 2].decode('utf-16-le', errors='replace').rstrip('\x00')

    def read_export_properties(self, export: ExportEntry) -> dict:
        """Read and parse the tagged properties from an export's serial data."""
        class_name = self.get_export_class_name(export)

        # Determine header size based on class type
        if self._is_actor_class(class_name):
            header_size = 32
        elif self._is_component_class(class_name):
            header_size = 8
        else:
            header_size = 4

        stream = self._get_data_stream()
        try:
            stream.seek(export.serial_offset)
            data = stream.read(export.serial_size)
        finally:
            if not isinstance(stream, io.BytesIO):
                stream.close()

        if len(data) <= header_size:
            return {}

        return self.parse_tagged_properties(data, header_size)

    def extract_actors(self, class_filter: set = None) -> list:
        """Extract all actors with their positions, rotations, and properties.

        Returns a list of dicts with: class_name, object_name, full_path,
        location (x,y,z), rotation (pitch,yaw,roll in degrees), draw_scale, properties.
        """
        actors = []
        stream = self._get_data_stream()
        needs_close = not isinstance(stream, io.BytesIO)

        try:
            for export in self.exports:
                class_name = self.get_export_class_name(export)

                if not self._is_actor_class(class_name):
                    continue
                if class_filter and class_name not in class_filter:
                    continue
                if export.serial_size <= 32:
                    continue

                stream.seek(export.serial_offset)
                data = stream.read(export.serial_size)
                props = self.parse_tagged_properties(data, 32)

                loc = props.get('Location', (0.0, 0.0, 0.0))
                rot_raw = props.get('Rotation', (0, 0, 0))

                # Convert rotation from UnrealRotator units to degrees
                rot_deg = tuple(v * 360.0 / 65536.0 for v in rot_raw) if isinstance(rot_raw, tuple) else (0, 0, 0)

                actor = {
                    'class_name': class_name,
                    'object_name': export.object_name,
                    'full_path': self.get_export_full_path(export),
                    'location': loc,
                    'rotation_deg': rot_deg,
                    'draw_scale': props.get('DrawScale', 1.0),
                    'draw_scale_3d': props.get('DrawScale3D', (1.0, 1.0, 1.0)),
                    'properties': props,
                }
                actors.append(actor)
        finally:
            if needs_close:
                stream.close()

        return actors

    def get_export_class_name(self, export: ExportEntry) -> str:
        """Get the class name for an export entry."""
        return self.resolve_object_name(export.class_index)

    def get_export_full_path(self, export: ExportEntry) -> str:
        """Build the full object path for an export (Package.Group.Name)."""
        parts = [export.object_name]
        current = export.package_index
        depth = 0
        while current != 0 and depth < 20:
            if current > 0:
                idx = current - 1
                if 0 <= idx < len(self.exports):
                    parts.insert(0, self.exports[idx].object_name)
                    current = self.exports[idx].package_index
                else:
                    break
            else:
                idx = -current - 1
                if 0 <= idx < len(self.imports):
                    parts.insert(0, self.imports[idx].object_name)
                    current = self.imports[idx].package_index
                else:
                    break
            depth += 1
        return '.'.join(parts)


def print_summary(pkg: PackageReader):
    """Print a human-readable package summary."""
    h = pkg.header
    print(f"=== Package: {os.path.basename(pkg.filepath)} ===")
    print(f"  Epic Version:     {h.epic_version}")
    print(f"  Licensee Version: {h.licensee_version}")
    print(f"  Header Size:      {h.total_header_size}")
    print(f"  Package Flags:    0x{h.package_flags:08X}")
    print(f"  GUID:             {h.guid[0]:08X}-{h.guid[1]:08X}-{h.guid[2]:08X}-{h.guid[3]:08X}")
    print(f"  Engine Version:   {h.engine_version}")
    print(f"  Cooker Version:   {h.cooker_version}")
    print(f"  Compression:      0x{h.compression_flags:08X} ({len(h.compressed_chunks)} chunks)")
    print(f"  Names:   {h.name_count:6d} (offset 0x{h.name_offset:08X})")
    print(f"  Exports: {h.export_count:6d} (offset 0x{h.export_offset:08X})")
    print(f"  Imports: {h.import_count:6d} (offset 0x{h.import_offset:08X})")
    if h.generations:
        print(f"  Generations: {len(h.generations)}")
        for i, (ec, nc, no) in enumerate(h.generations):
            print(f"    [{i}] exports={ec}, names={nc}, net_objects={no}")
    print()


def print_names(pkg: PackageReader, limit: int = 50):
    """Print name table entries."""
    print(f"--- Name Table ({len(pkg.names)} entries) ---")
    for i, n in enumerate(pkg.names):
        if limit and i >= limit:
            print(f"  ... ({len(pkg.names) - limit} more)")
            break
        flags_str = f" [0x{n.flags:016X}]" if n.flags else ""
        print(f"  [{i:4d}] {n.name}{flags_str}")
    print()


def print_imports(pkg: PackageReader, limit: int = 50):
    """Print import table entries."""
    print(f"--- Import Table ({len(pkg.imports)} entries) ---")
    for i, imp in enumerate(pkg.imports):
        if limit and i >= limit:
            print(f"  ... ({len(pkg.imports) - limit} more)")
            break
        print(f"  [{i:4d}] {imp.class_package}.{imp.class_name} '{imp.object_name}' (outer={imp.package_index})")
    print()


def print_exports(pkg: PackageReader, limit: int = 50):
    """Print export table entries."""
    print(f"--- Export Table ({len(pkg.exports)} entries) ---")
    for i, exp in enumerate(pkg.exports):
        if limit and i >= limit:
            print(f"  ... ({len(pkg.exports) - limit} more)")
            break
        class_name = pkg.get_export_class_name(exp)
        full_path = pkg.get_export_full_path(exp)
        comp_str = f" components={len(exp.component_map)}" if exp.component_map else ""
        print(f"  [{i:4d}] {class_name:30s} {full_path:50s} "
              f"size={exp.serial_size:8d} offset=0x{exp.serial_offset:08X}"
              f" flags=0x{exp.object_flags:016X}{comp_str}")
    print()


def print_class_histogram(pkg: PackageReader):
    """Print export class distribution."""
    from collections import Counter
    classes = Counter()
    for exp in pkg.exports:
        classes[pkg.get_export_class_name(exp)] += 1
    print(f"--- Class Distribution ({len(classes)} classes) ---")
    for cls, count in classes.most_common(30):
        print(f"  {cls:40s} {count:6d}")
    print()


def main():
    if len(sys.argv) < 2:
        print("Usage: python upk_parser.py <file.upk|file.umap> [--names N] [--imports N] [--exports N] [--all]")
        print("  --names N    Show first N name entries (default 50, 0=all)")
        print("  --imports N  Show first N import entries (default 50, 0=all)")
        print("  --exports N  Show first N export entries (default 50, 0=all)")
        print("  --all        Show all entries (equivalent to --names 0 --imports 0 --exports 0)")
        print("  --classes    Show class distribution histogram")
        sys.exit(1)

    filepath = sys.argv[1]
    names_limit = 50
    imports_limit = 50
    exports_limit = 50
    show_classes = False

    i = 2
    while i < len(sys.argv):
        arg = sys.argv[i]
        if arg == '--names' and i + 1 < len(sys.argv):
            names_limit = int(sys.argv[i + 1])
            i += 2
        elif arg == '--imports' and i + 1 < len(sys.argv):
            imports_limit = int(sys.argv[i + 1])
            i += 2
        elif arg == '--exports' and i + 1 < len(sys.argv):
            exports_limit = int(sys.argv[i + 1])
            i += 2
        elif arg == '--all':
            names_limit = 0
            imports_limit = 0
            exports_limit = 0
            i += 1
        elif arg == '--classes':
            show_classes = True
            i += 1
        else:
            print(f"Unknown argument: {arg}")
            sys.exit(1)

    pkg = PackageReader(filepath)
    try:
        pkg.parse()
    except Exception as e:
        print(f"ERROR parsing {filepath}: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

    print_summary(pkg)
    print_names(pkg, limit=names_limit)
    print_imports(pkg, limit=imports_limit)
    print_exports(pkg, limit=exports_limit)
    if show_classes:
        print_class_histogram(pkg)

    print(f"Successfully parsed {os.path.basename(filepath)}: "
          f"{len(pkg.names)} names, {len(pkg.imports)} imports, {len(pkg.exports)} exports")


if __name__ == '__main__':
    main()
