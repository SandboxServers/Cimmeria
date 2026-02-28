function Get-ProjectPaths {
    <#
    .SYNOPSIS
        Returns a hashtable of canonical project paths resolved from $script:ProjectRoot.
    #>
    return @{
        ProjectRoot  = $script:ProjectRoot
        ExternalDir  = Join-Path $script:ProjectRoot "external"
        DownloadDir  = Join-Path $script:ProjectRoot "external\_downloads"
        BootstrapDir = Join-Path $script:ProjectRoot "bootstrap"
        PatchDir     = Join-Path $script:ProjectRoot "bootstrap\patches"
        TemplateDir  = Join-Path $script:ProjectRoot "bootstrap\templates"
        ConfigDir    = Join-Path $script:ProjectRoot "config"
        DbDir        = Join-Path $script:ProjectRoot "db"
        ServerDir    = Join-Path $script:ProjectRoot "server"
    }
}
