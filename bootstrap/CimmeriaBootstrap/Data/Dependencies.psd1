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
}
