@{
    PostgreSQL = @{
        Version   = "17.9"
        Windows   = @{
            Url      = "https://get.enterprisedb.com/postgresql/postgresql-17.9-1-windows-x64-binaries.zip"
            FileName = "postgresql-17.9-x64-binaries.zip"
        }
        Linux     = @{
            # On Linux, PostgreSQL is installed via system package manager
            PackageName = "postgresql-17"
        }
        MacOS     = @{
            # On macOS, PostgreSQL is installed via Homebrew
            BrewFormula = "postgresql@17"
        }
        Type      = "prebuilt"
        Notes     = "17.9 is the latest 17.x patch release (Feb 2026). EOL Nov 2029."
    }

    Recast = @{
        Version   = "1.6.0"
        Url       = "https://github.com/recastnavigation/recastnavigation/archive/refs/tags/v1.6.0.zip"
        FileName  = "recastnavigation-1.6.0.zip"
        Type      = "source"
        Notes     = "Detour navmesh query library. Compiled from source via cc crate during cargo build. DT_NAVMESH_VERSION=7."
    }

    SevenZip = @{
        Version       = "26.00"
        ExtraUrl      = "https://github.com/ip7z/7zip/releases/download/26.00/7z2600-extra.7z"
        ExtraFileName = "7z2600-extra.7z"
        # 7zr.exe is a standalone binary that can extract .7z archives (bootstrap only)
        BootstrapUrl  = "https://www.7-zip.org/a/7zr.exe"
        Notes         = "7-Zip Extra: standalone 7za.exe console version. Bundled with sgw-launcher."
    }

    # =============================================================================
    # Reverse-Engineering Toolchain (opt-in via -WithReToolchain)
    # =============================================================================
    # See docs/guides/re-toolchain-setup.md for the developer-facing walkthrough.

    Ghidra = @{
        Version  = "12.0.4"
        BuildTag = "Ghidra_12.0.4_build"
        # Public release zip from the NSA's official GitHub repo.
        Url      = "https://github.com/NationalSecurityAgency/ghidra/releases/download/Ghidra_12.0.4_build/ghidra_12.0.4_PUBLIC_20260303.zip"
        FileName = "ghidra_12.0.4_PUBLIC_20260303.zip"
        # Sha256 from the GitHub release page. Verified manually; update on version bump.
        Sha256   = "c3b458661d69e26e203d739c0c82d143cc8a4a29d9e571f099c2cf4bda62a120"
        Type     = "prebuilt"
        Notes    = "Static analyzer. Requires JDK 21+. Extracted to ghidra/ghidra_12.0.4_PUBLIC/."
    }

    GhidraMcp = @{
        # bethington/ghidra-mcp — Python bridge + Ghidra extension plugin.
        # Pin a version whose pre-built extension zip targets the Ghidra version
        # above. v5.10.0 was confirmed working against Ghidra 12.0.4 in May 2026.
        # Bump only after verifying the new extension zip targets your Ghidra.
        Version       = "5.10.0"
        RepoUrl       = "https://github.com/bethington/ghidra-mcp.git"
        # Release zip with the pre-built extension. Saves having to install Maven.
        ExtensionUrl  = "https://github.com/bethington/ghidra-mcp/releases/download/v5.10.0/GhidraMCP-5.10.0.zip"
        ExtensionFile = "GhidraMCP-5.10.0.zip"
        Type          = "source+prebuilt"
        Notes         = "Cloned to external/ghidra-mcp/. Extension zip deployed to %APPDATA%\ghidra\ghidra_<version>_PUBLIC\Extensions\GhidraMCP\."
    }

    X64Dbg = @{
        # x64dbg ships rolling snapshots — the snapshot release tag is reused
        # but the asset filename embeds a build timestamp that changes weekly.
        # Rather than pin a URL that 404s on rollover, the bootstrap checks for
        # an existing dbg\release\x96dbg.exe and falls back to printing a manual
        # download hint when the cached zip isn't present.
        SnapshotPageUrl = "https://github.com/x64dbg/x64dbg/releases/tag/snapshot"
        Type            = "manual-fetch"
        Notes           = "Dynamic analyzer. Extracted to dbg/. Use x96dbg.exe — it dispatches to x32dbg or x64dbg based on target bitness. Manual download because snapshot asset URLs rotate."
    }

    X64DbgAutomate = @{
        # dariushoule/x64dbg-automate — x64dbg plugin (.dp32/.dp64) + Python MCP server.
        Version     = "0.6.1"
        ReleaseTag  = "v0.6.1-green_pepe"
        # Plugin DLLs and the libzmq runtime they need.
        Dp32Url     = "https://github.com/dariushoule/x64dbg-automate/releases/download/v0.6.1-green_pepe/x64dbg-automate.dp32"
        Dp64Url     = "https://github.com/dariushoule/x64dbg-automate/releases/download/v0.6.1-green_pepe/x64dbg-automate.dp64"
        ZmqDllUrl   = "https://github.com/dariushoule/x64dbg-automate/releases/download/v0.6.1-green_pepe/libzmq-mt-4_3_5.dll"
        # PyPI package — installed into .venvs/x64dbg-mcp/.
        PyPiPackage = "x64dbg-automate"
        Type        = "prebuilt+pypi"
        Notes       = "Plugin DLLs go into dbg/release/{x32,x64}/plugins/. PyPI package provides the x64dbg-automate-mcp entrypoint."
    }
}
