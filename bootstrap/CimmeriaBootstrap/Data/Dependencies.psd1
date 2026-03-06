@{
    Boost = @{
        Version           = "1.55.0"
        VersionUnderscore = "1_55_0"
        Url               = "https://archives.boost.io/release/1.55.0/source/boost_1_55_0.tar.gz"
        FileName          = "boost_1_55_0.tar.gz"
        Type              = "source+build"
        ExtractTo         = "boost"
        Notes             = "Full source tree. Built via b2 with msvc toolset."
    }

    OpenSSL = @{
        Version   = "1.0.1e"
        Url       = "https://github.com/openssl/openssl/archive/refs/tags/OpenSSL_1_0_1e.tar.gz"
        FileName  = "openssl-1.0.1e.tar.gz"
        Type      = "source+build"
        ExtractTo = "openssl_src"
        Notes     = "Needs Perl + MSVC to build. Produces headers + .lib files."
    }

    PostgreSQL = @{
        Version   = "17.9"
        Url       = "https://get.enterprisedb.com/postgresql/postgresql-17.9-1-windows-x64-binaries.zip"
        FileName  = "postgresql-17.9-x64-binaries.zip"
        Type      = "prebuilt"
        ExtractTo = "postgresql_pkg"
        Notes     = "17.9 is the latest 17.x patch release (Feb 2026). EOL Nov 2029. Upgrade from 9.2 requires pgdata re-init."
    }

    Python = @{
        Version    = "3.4.1"
        Url64      = "https://www.python.org/ftp/python/3.4.1/python-3.4.1.amd64.msi"
        Url32      = "https://www.python.org/ftp/python/3.4.1/python-3.4.1.msi"
        FileName64 = "python-3.4.1.amd64.msi"
        FileName32 = "python-3.4.1.msi"
        Type       = "installer-extract"
        ExtractTo  = "python"
        Notes      = "MSI contains headers (include/), import libs (libs/), and python34.dll."
    }

    SOCI = @{
        Version   = "3.2.1"
        Url       = "https://github.com/SOCI/soci/archive/refs/tags/3.2.1.tar.gz"
        FileName  = "soci-3.2.1.tar.gz"
        Type      = "source"
        ExtractTo = "soci"
        Notes     = "Source tree used directly for headers. Libs built separately via build-soci.bat."
    }

    SDL = @{
        Version   = "1.2.15"
        Url       = "https://www.libsdl.org/release/SDL-devel-1.2.15-VC.zip"
        FileName  = "SDL-devel-1.2.15-VC.zip"
        Type      = "prebuilt"
        ExtractTo = "SDL"
        Notes     = "Pre-built Visual C++ development libraries."
    }

    Recast = @{
        Version   = "git-2014"
        Url       = "https://github.com/recastnavigation/recastnavigation/archive/refs/heads/main.zip"
        FileName  = "recastnavigation-main.zip"
        Type      = "source"
        ExtractTo = "recast"
        Notes     = "Using main branch; compiled by Recast.vcxproj in the solution."
    }

    BigWorld191 = @{
        Version   = "1.9.1"
        Url       = "https://github.com/v2v3v4/BigWorld-Engine-1.9.1/archive/refs/heads/master.zip"
        FileName  = "BigWorld-Engine-1.9.1.zip"
        Type      = "source"
        ExtractTo = "bigworld_1.9.1"
        Notes     = "Reference source for Mercury protocol, entity system, and server architecture."
    }

    Qt = @{
        Version   = "5.15.18"
        Url       = "https://github.com/martinrotter/qt-minimalistic-builds/releases/download/5.15.18/qt-5.15.18-dynamic-msvc2022-x86_64.7z"
        FileName  = "qt-5.15.18-dynamic-msvc2022-x86_64.7z"
        Type      = "prebuilt"
        ExtractTo = "qt"
        Notes     = "Prebuilt Qt 5.15.18 (MSVC2022, x64). Needed for ServerEd tool."
    }

    BigWorld201 = @{
        Version   = "2.0.1"
        Url       = "https://github.com/v2v3v4/BigWorld-Engine-2.0.1/archive/refs/heads/master.zip"
        FileName  = "BigWorld-Engine-2.0.1.zip"
        Type      = "source"
        ExtractTo = "bigworld_2.0.1"
        Notes     = "Reference source for FixedDictDataType, UserDataType, and later protocol features."
    }
}
