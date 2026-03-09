@{
    RootModule        = 'CimmeriaBootstrap.psm1'
    ModuleVersion     = '2.0.0'
    GUID              = 'a3f7d8e2-4b1c-4e9a-b5d6-8c2f1a3e7b90'
    Author            = 'Cimmeria Project'
    Description       = 'Bootstrap module for the Cimmeria (Stargate Worlds Emulator) project. Builds the Rust server, manages PostgreSQL, and handles server lifecycle.'
    PowerShellVersion = '7.0'

    FunctionsToExport = @(
        'Install-CimmeriaDependencies',
        'Build-CimmeriaServer',
        'Build-CimmeriaApp',
        'Build-CimmeriaLauncher',
        'Initialize-CimmeriaDatabase',
        'Start-CimmeriaServer',
        'Stop-CimmeriaServer',
        'Update-CimmeriaClient',
        'Invoke-CimmeriaBootstrap'
    )

    CmdletsToExport   = @()
    VariablesToExport  = @()
    AliasesToExport    = @()
}
