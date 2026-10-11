function Test-LauncherAsset {
    <#
    .SYNOPSIS
        Whether a release asset exists, by a HEAD request. $false on a 404;
        throws on any other failure.
    #>
    param([Parameter(Mandatory)][string]$Url)

    try {
        Invoke-WebRequest -Uri $Url -Method Head -UseBasicParsing | Out-Null
        return $true
    } catch {
        $response = $_.Exception.Response
        if ($null -ne $response -and [int]$response.StatusCode -eq 404) {
            return $false
        }
        throw "Checking $Url failed: $($_.Exception.Message)"
    }
}
