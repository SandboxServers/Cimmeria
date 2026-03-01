function Get-DownloadFile {
    <#
    .SYNOPSIS
        Downloads a file from a URL if it doesn't already exist locally.
    #>
    param(
        [Parameter(Mandatory)][string]$Url,
        [Parameter(Mandatory)][string]$OutPath
    )

    if (Test-Path $OutPath) {
        $size = Format-FileSize (Get-Item $OutPath).Length
        Write-Status "Already downloaded: $(Split-Path -Leaf $OutPath) ($size)" "DarkGray"
        return
    }

    Write-Status "Downloading: $Url"
    Write-Status "-> $OutPath"

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $prevPref = $ProgressPreference
    $ProgressPreference = 'SilentlyContinue'
    try {
        Invoke-WebRequest -Uri $Url -OutFile $OutPath -UseBasicParsing
    } finally {
        $ProgressPreference = $prevPref
    }
    $sw.Stop()
    $size = Format-FileSize (Get-Item $OutPath).Length
    Write-Status "Done ($size in $("{0:N1}" -f $sw.Elapsed.TotalSeconds)s)" "Green"
}
