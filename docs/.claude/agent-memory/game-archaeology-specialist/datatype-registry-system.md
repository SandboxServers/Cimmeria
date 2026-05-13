---
name: datatype-registry-system
description: BigWorld DataType two-registry model, MD5 hashing, CME property system, all confirmed by W-entity-desc-B (2026-05-13).
metadata:
  type: project
---

## DataType Two-Registry System (confirmed 2026-05-13)

Two `std::map<string, DataType*>` registries with distinct roles:

| Symbol | Address | Populated By | Queried By | Role |
|--------|---------|-------------|------------|------|
| `g_mapDataTypeRegistry` | `DAT_01f126b8` | `DataType_RegisterBuiltins` @ `0x01596c40` | `DataType_BuildFromSection` @ `0x01597150` | BUILD path — alias.xml tag → DataType* factory |
| `g_pMetaDataTypeRegistry` | `DAT_01f126b4` | `DataType_Register` @ `0x01597ce0` | `DataType_LookupByName` @ `0x01595f00` | LOOKUP path — C++ type name → SimpleMetaDataType<T>* |

**W4-B2 ambiguity resolved**: Both `g_pMetaDataTypeRegistry` and `g_mapDataTypeRegistryLookup` in the W4-B2 checkpoint referred to the SAME address `01f126b4`. Only two registries exist.

## DataType Subclass Hierarchy

17 primitive types, each with 4-function group (DtorBody, Constructor, GetTypeName_WriteStream, New).
SimpleMetaDataType<T> constructors: `0x0159db10`–`0x0159e510` (17 functions, each calls DataType_Register).

| Type | Constructor Address |
|------|---------------------|
| UInt8 (IntegerDataType<unsigned char>) | `0x01599150` |
| Int8 (IntegerDataType<char>) | `0x015995f0` |
| UInt16 (IntegerDataType<unsigned short>) | `0x01599340` |
| Int16 (IntegerDataType<short>) | `0x015997d0` |
| Int32 (IntegerDataType<long>) | `0x015999b0` |
| UInt32 (LongIntegerDataType<unsigned long>) | `0x01599b90` |
| Int64 (LongIntegerDataType<__int64>) | `0x01599d90` |
| UInt64 (LongIntegerDataType<unsigned __int64>) | `0x01599f70` |
| Float (FloatDataType) | `0x0159a220` |
| String (StringDataType) | `0x0159a3f0` |
| WideString (WideStringDataType) | `0x0159a5e0` |
| Python (PythonDataType) | `0x0159a790` |
| Vector2 (VectorDataType<Vector2>) | `0x0159aa00` |
| Vector3 (VectorDataType<Vector3>) | `0x0159acf0` |
| Vector4 (VectorDataType<Vector4>) | `0x0159af80` |
| Blob (BlobDataType) | `0x0159b300` |
| MailBox (MailBoxDataType) | `0x0159b510` |

## MD5 Type Signature System

| Address | Function |
|---------|----------|
| `0x015a3d70` | MD5_Init — constants 0x67452301/0xefcdab89/0x98badcfe/0x10325476 |
| `0x015a3da0` | MD5_Update (wrapper) |
| `0x015a3c00` | MD5_Update_Block (core) |
| `0x015a3cd0` | MD5_Finalize |
| `0x015a3de0` | MD5_DigestToHexString — `"0123456789ABCDEF"` at DAT_01b1bd40 |

Each DataType's GetTypeName_WriteStream feeds binary type encoding into MD5 for protocol versioning fingerprints.

## CME Property System

`CME::BasicPropertyList<TypeList<14 types>>` — secondary typed property system parallel to BigWorld stream protocol.
TypeList: uint8, int8, uint16, int16, uint32, int32, uint64, int64, float, wstring, Vector2, Vector3, Vector4, NullType.

Key functions:
- `CMEPropertyList_StreamToTree` @ `0x0159bcd0`
- `CMEPropertyList_PrintToStream` @ `0x0159bd70`
- `CMEBasicPropertyList_StreamToTree` @ `0x015a27f0`
- `CMEBasicPropertyList_PrintToStream` @ `0x015a2880`

## Sub-Slot Encoding Status

CONFIRMED from W4-B1. Threshold 0x3e=62, functions at `0x01590df0` (Assign) and `0x01590ee0` (Decode).
SGWPlayer has 157 total client methods; sub-slot kicks in at index 62+.

## High-Half Scan Result (015a8b40–015bffe0)

This entire range is NON-PROTOCOL developer tooling:
- PE debug utilities (015a8b40–015a9510): PE relocation/import walkers, MZ module loaders
- CME::Win32ThreadEx (015a9750–015a9a80): Thread stop/wait/close; source .\\thread\\win32\\win32_thread.cpp
- Unreal Editor GFx importer (015a9d00–015aac70): wxDirDialog, GFXExport.exe launcher, batch .swf importer
- UnrealRPC SpawnPoint/PointSet/AvatarSet API (015abfd0–015bfe70): 170+ XML SOAP serializer stubs + RPC handlers. SpecSelection struct = avatar appearance (setExists, colors, bodySetName, bodyComponentNames). vftable confirmed: `UnRPC::unrpc__SpawnPointCreateData::vftable` @ `015b8980`. Master dispatch @ `015b2410` (type codes 0x01–0xa5).

**Why:** These are all editor/level-design backend tools compiled into the client binary for developer builds. Not part of the live game protocol.
