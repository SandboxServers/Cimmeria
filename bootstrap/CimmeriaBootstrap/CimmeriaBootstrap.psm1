$script:ScriptStartTime = Get-Date
$script:ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$script:IsWSL = (Test-Path variable:IsLinux) -and $IsLinux -and (Test-Path /proc/version) -and ((Get-Content /proc/version -Raw) -match 'microsoft|WSL')

# Dot-source all private functions
Get-ChildItem (Join-Path $PSScriptRoot "Private/*.ps1") | ForEach-Object { . $_.FullName }

# Dot-source all public functions
Get-ChildItem (Join-Path $PSScriptRoot "Public/*.ps1") | ForEach-Object { . $_.FullName }
