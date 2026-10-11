# Client-patch generators

Generators for files shipped in client patches under `data/client-patches/`.

| Entry point | What it does |
|---|---|
| `gm_slash_commands.py` | Generate patch 012's `InternalSlashCommands.xml` from `data/client-patches/012-gm-slash-commands/commands.toml`; `--check` exits 1 on drift (CI runs it) |
| `cme_prose_hashes.py` | Write the SHA-256 list the no-CME-text guard for patch 012 checks against |

The patch sets, how they are built and how the launcher delivers them: [data/client-patches/README.md](../../data/client-patches/README.md#012-gm-slash-commands) and [client-patches.md](../../docs/architecture/client-patches.md).
