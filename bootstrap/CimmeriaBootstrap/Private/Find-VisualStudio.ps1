function Find-VisualStudio {
    <#
    .SYNOPSIS
        Detects Visual Studio with C++ tools via vswhere.exe.
        Returns a hashtable with InstallPath, VCToolsVersion, ClExePath, DisplayName, Version.
        Returns $null if not found.
    #>
    param(
        [switch]$InstallVS
    )

    if (-not ($IsWindows -or (-not (Test-Path variable:IsWindows)))) {
        Write-Status "Not running on Windows - skipping VS detection" "DarkGray"
        return $null
    }

    $vswhereExe = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhereExe)) {
        $vswhereExe = "$env:ProgramFiles\Microsoft Visual Studio\Installer\vswhere.exe"
    }

    $result = @{
        InstallPath    = $null
        VCToolsVersion = $null
        ClExePath      = $null
        DisplayName    = $null
        Version        = $null
    }

    if (Test-Path $vswhereExe) {
        $vsInfo = & $vswhereExe -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
        if ($vsInfo) {
            $result.InstallPath = $vsInfo.Trim()
        } else {
            $vsInfo = & $vswhereExe -latest -property installationPath 2>$null
            if ($vsInfo) {
                $result.InstallPath = $vsInfo.Trim()
                Write-Status "WARNING: VS found but C++ tools may not be installed" "Yellow"
            }
        }

        if ($result.InstallPath) {
            $result.DisplayName = (& $vswhereExe -latest -property displayName 2>$null)
            $result.Version = (& $vswhereExe -latest -property installationVersion 2>$null)
            Write-Status "Found: $($result.DisplayName) (v$($result.Version))" "Green"
            Write-Status "Path:  $($result.InstallPath)" "DarkGray"

            $vcToolsDir = Join-Path $result.InstallPath "VC\Tools\MSVC"
            if (Test-Path $vcToolsDir) {
                $result.VCToolsVersion = (Get-ChildItem $vcToolsDir -Directory | Sort-Object Name -Descending | Select-Object -First 1).Name
                Write-Status "MSVC Tools: $($result.VCToolsVersion)" "DarkGray"

                $clPath = Join-Path $vcToolsDir "$($result.VCToolsVersion)\bin\Hostx64\x64\cl.exe"
                if (Test-Path $clPath) {
                    $result.ClExePath = $clPath
                    Write-Status "cl.exe: $($result.ClExePath)" "DarkGray"
                }
            }
            return $result
        }
    }

    if ($InstallVS) {
        Write-Status "Visual Studio not found. Downloading VS Community installer..." "Yellow"
        $vsInstallerUrl = "https://aka.ms/vs/18/release/vs_community.exe"
        $vsInstallerPath = Join-Path $env:TEMP "vs_community.exe"

        $prevPref = $ProgressPreference
        $ProgressPreference = 'SilentlyContinue'
        try {
            Invoke-WebRequest -Uri $vsInstallerUrl -OutFile $vsInstallerPath -UseBasicParsing
        } finally {
            $ProgressPreference = $prevPref
        }

        Write-Status "Running VS Community installer (this may take 10-30 minutes)..." "Yellow"
        Write-Status "Installing workloads: C++ Desktop Development" "DarkGray"

        $vsArgs = @(
            "--add", "Microsoft.VisualStudio.Workload.NativeDesktop",
            "--add", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "--add", "Microsoft.VisualStudio.Component.Windows11SDK.26100",
            "--passive", "--norestart", "--wait"
        )
        $vsProc = Start-Process -FilePath $vsInstallerPath -ArgumentList $vsArgs -Wait -PassThru
        if ($vsProc.ExitCode -ne 0 -and $vsProc.ExitCode -ne 3010) {
            throw "VS installer exited with code $($vsProc.ExitCode). Check the installer log for details."
        }

        Write-Host ""
        Write-Host "Visual Studio Community installed successfully." -ForegroundColor Green
        Write-Host "IMPORTANT: Restart your PowerShell session, then re-run this script." -ForegroundColor Yellow
        Write-Host "  pwsh setup.ps1" -ForegroundColor White
        exit 0
    }

    Write-Host ""
    Write-Host "ERROR: Visual Studio with C++ tools not found." -ForegroundColor Red
    Write-Host ""
    Write-Host "Options:" -ForegroundColor White
    Write-Host "  1. Install manually from: https://visualstudio.microsoft.com/downloads/" -ForegroundColor Gray
    Write-Host "     Select 'Desktop development with C++' workload" -ForegroundColor Gray
    Write-Host ""
    Write-Host "  2. Re-run this script with -InstallVS to auto-install VS Community:" -ForegroundColor Gray
    Write-Host "     pwsh setup.ps1 -InstallVS" -ForegroundColor White
    return $null
}
