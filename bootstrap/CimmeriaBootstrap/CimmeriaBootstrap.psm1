$script:ScriptStartTime = Get-Date
$script:ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path

# Dot-source all private functions
Get-ChildItem (Join-Path $PSScriptRoot "Private/*.ps1") | ForEach-Object { . $_.FullName }

# Dot-source all public functions
Get-ChildItem (Join-Path $PSScriptRoot "Public/*.ps1") | ForEach-Object { . $_.FullName }
