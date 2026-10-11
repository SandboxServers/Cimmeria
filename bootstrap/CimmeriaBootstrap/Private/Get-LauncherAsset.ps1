function Get-LauncherAsset {
    <#
    .SYNOPSIS
        Downloads one release asset over any copy already there. Returns $false
        on a 404 and throws on any other failure.
    #>
    param(
        [Parameter(Mandatory)][string]$Url,
        [Parameter(Mandatory)][string]$OutPath
    )

    Write-Status "Downloading: $Url"
    $prevPref = $ProgressPreference
    $ProgressPreference = 'SilentlyContinue'
    try {
        Invoke-WebRequest -Uri $Url -OutFile $OutPath -UseBasicParsing
        return $true
    } catch {
        Remove-Item -LiteralPath $OutPath -Force -ErrorAction SilentlyContinue
        $response = $_.Exception.Response
        if ($null -ne $response -and [int]$response.StatusCode -eq 404) {
            return $false
        }
        throw "Download of $Url failed: $($_.Exception.Message)"
    } finally {
        $ProgressPreference = $prevPref
    }
}
