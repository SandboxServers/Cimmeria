@{
    RootModule        = 'CimmeriaBootstrap.psm1'
    ModuleVersion     = '1.0.0'
    GUID              = 'a3f7d8e2-4b1c-4e9a-b5d6-8c2f1a3e7b90'
    Author            = 'Cimmeria Project'
    Description       = 'Bootstrap module for the Cimmeria (Stargate Worlds Emulator) project. Downloads, builds, and manages all dependencies and server lifecycle.'
    PowerShellVersion = '7.0'

    FunctionsToExport = @(
        'Install-CimmeriaDependencies',
        'Build-CimmeriaLibraries',
        'Build-CimmeriaSolution',
        'Initialize-CimmeriaDatabase',
        'Initialize-CimmeriaRuntime',
        'Start-CimmeriaServer',
        'Stop-CimmeriaServer',
        'Patch-CimmeriaClient',
        'Invoke-CimmeriaBootstrap'
    )

    CmdletsToExport   = @()
    VariablesToExport  = @()
    AliasesToExport    = @()
}
