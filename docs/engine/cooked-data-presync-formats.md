---
title: "Cooked-data presync formats"
type: reference
audience: engineers
last_updated: 2026-10-26
---

# Cooked-data presync formats

Reference for the bytes the launcher presync reads and writes. The behaviour around them, the routes and the operator notes are in [cooked-data-pipeline.md § Launcher presync](cooked-data-pipeline.md#launcher-presync). The code is the `cimmeria-cooked-pak` crate (`crates/cooked-pak`).

## The PAK `write_pak` produces

`write_pak(out, version, entries)` writes a PAK the client reads the same as a cooked one:

- one `_<key>` entry per record, in **ascending key order**, deflate-compressed, each with the default zip timestamp (`zip::DateTime::default()`), so the same input always gives the same bytes;
- then `MetaData` **last**, stored (not compressed), exactly 4 bytes, the version as little-endian `u32`.

`read_pak` accepts only entry names of the form `_<u32>` (the rule `ResourceCache::load_pak` uses) and ignores any other name. `read_version` returns the first 4 bytes of `MetaData` as LE, or `None` when the entry is absent or shorter than 4 bytes. `cooked-pak-tool layout --pak <file>` lists a PAK's entries in order.

## Index JSON

`GET /api/cooked/v1/index` returns:

```json
{"schema":1,"categories":[{"category_id":4,"pak":"CookedDataItems.pak","version":123,"entries":5000,"changed":12,"body_bytes":40960,"digest":"<64 lower-case hex>"}]}
```

| Field | Meaning |
|---|---|
| `schema` | Always `1`. The reader ignores unknown fields. |
| `category_id` | The wire category id (1 to 21). |
| `pak` | The category's PAK file name, which is also the name the launcher writes. |
| `version` | The served `MetaData` version. |
| `entries` | Number of keys in the served category. |
| `changed` | Number of entries whose bytes the body carries. |
| `body_bytes` | Size of the `CPS1` body in bytes. |
| `digest` | The `CPD1` digest of the whole served category, as 64 lower-case hex digits. |

Categories are listed in ascending id order.

## `CPS1` body

`GET /api/cooked/v1/category/{id}?version=<v>` returns one category. All integers are `u32` little-endian:

```text
"CPS1" | category_id | version | key_count
       | key_count x key            (strictly ascending)
       | changed_count
       | changed_count x (key | len | bytes)   (strictly ascending; each key also in the key list)
       | digest (32 bytes)
```

Nothing follows the digest. The decoder checks every count and length against fixed limits **before** it allocates: at most 1,000,000 keys, 4 MiB per entry and 32 MiB in total. A body that breaks a rule is refused (bad magic, truncated, keys not ascending, a changed key not in the key list, too large, trailing bytes).

To rebuild a category the launcher takes, for each key in the key list, the changed bytes if the body has them, else the same key from the player's own `SourceCache.en-us` PAK. Keys in the source PAK but not in the list are dropped. If any key is in neither place the compose fails (`base_missing`). Then it computes the digest over the result and compares it to the body's; a difference is `digest_mismatch`.

## `CPD1` digest

SHA-256 over:

```text
"CPD1" | category_id (u32 LE) | version (u32 LE) | count (u32 LE)
       | count x (key (u32 LE) | len (u32 LE) | bytes)      (keys ascending)
```

Because the digest covers the **served** category (the whole thing, not just the changed entries), it ties the composed result to what the server would have served, whatever base the player started from.

> [!IMPORTANT]
> **The digest is integrity, not authenticity.** It catches a corrupted or truncated download and a source PAK that does not match the server's base. It does not prove the server is the one the player meant to reach: the routes are plain HTTP on the login port, the same trust level as the SOAP and Mercury channel to that server (D-LP10). HTTPS through the proxy is a tracked follow-up (LP-F2). Do not describe the digest as a signature.

## Which version the client sees

The client copies `SourceCache.en-us` over `Cache.en-US` at startup when the source version is newer, so the launcher decides per category from the cache and source versions:

- the *effective* version is the cache's, unless there is no cache PAK or the source is newer, in which case it is the source's;
- if the effective version equals the served version the category is up to date and is not downloaded;
- if the source version is newer than the served one the launcher cannot fix it (the client would overwrite the write at startup) and fails with `source_newer` before any download (D-LP6, D-LP18);
- otherwise the launcher writes the composed PAK.

Versions compare as unsigned `u32` (D-LP15). The launcher writes only the Documents `Cache.en-US`, never `SourceCache.en-us` or the install.
