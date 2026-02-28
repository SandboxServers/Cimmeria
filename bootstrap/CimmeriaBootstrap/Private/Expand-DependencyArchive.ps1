function Expand-DependencyArchive {
    <#
    .SYNOPSIS
        Extracts an archive (.tar.gz, .zip, .msi, .7z) into a destination directory.
    #>
    param(
        [Parameter(Mandatory)][string]$ArchivePath,
        [Parameter(Mandatory)][string]$DestDir
    )

    $fileName = Split-Path -Leaf $ArchivePath
    $size = Format-FileSize (Get-Item $ArchivePath).Length
    Write-Status "Extracting: $fileName ($size) -> $(Split-Path -Leaf $DestDir)"

    $sw = [System.Diagnostics.Stopwatch]::StartNew()

    if ($ArchivePath -match '\.7z$') {
        $7z = Get-Command 7z -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
        if (-not $7z -and $IsWindows) {
            $7z = Join-Path $env:ProgramFiles "7-Zip\7z.exe"
            if (-not (Test-Path $7z)) { $7z = "7z" }
        }
        if (-not $7z) { $7z = "7z" }
        Write-Status "  Using 7z..." "DarkGray"
        & $7z x $ArchivePath -o"$DestDir" -y 2>&1 | ForEach-Object {
            if ($_ -match 'Extracting|Everything is Ok') { Write-Status "  $_" "DarkGray" }
        }
    }
    elseif ($ArchivePath -match '\.tar\.gz$') {
        Write-Status "  Using tar (this may take a while for large archives)..." "DarkGray"
        tar -xzvf $ArchivePath -C $DestDir 2>&1 | ForEach-Object -Begin { $count = 0 } -Process {
            $count++
            if ($count % 2000 -eq 0) { Write-Status "  ... $count files extracted" "DarkGray" }
        }
        Write-Status "  Total: $count files extracted" "DarkGray"
    }
    elseif ($ArchivePath -match '\.zip$') {
        Write-Status "  Using Expand-Archive..." "DarkGray"
        Expand-Archive -Path $ArchivePath -DestinationPath $DestDir -Force
    }
    elseif ($ArchivePath -match '\.msi$') {
        $extractDir = Join-Path $DestDir "msi_extract"
        New-Item -ItemType Directory -Path $extractDir -Force | Out-Null
        if ($IsWindows) {
            Write-Status "  Using msiexec..." "DarkGray"
            msiexec /a "$ArchivePath" /qn TARGETDIR="$extractDir" | Out-Null
        } else {
            $msiextractCmd = Get-Command msiextract -ErrorAction SilentlyContinue
            if ($msiextractCmd) {
                Write-Status "  Using msiextract..." "DarkGray"
                msiextract -C $extractDir $ArchivePath 2>&1 | ForEach-Object {
                    Write-Status "  $_" "DarkGray"
                }
            } else {
                throw "msiextract not found. Install it with: sudo apt install msitools"
            }
        }
    }

    $sw.Stop()
    Write-Status "Extraction complete ($("{0:N1}" -f $sw.Elapsed.TotalSeconds)s)" "Green"
}
