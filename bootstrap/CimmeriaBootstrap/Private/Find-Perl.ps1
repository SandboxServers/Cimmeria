function Find-Perl {
    <#
    .SYNOPSIS
        Finds a Perl installation (Strawberry Perl preferred, falls back to Git's bundled Perl).
        Returns the path to perl.exe, or $null if not found.
    #>

    if (-not ($IsWindows -or (-not (Test-Path variable:IsWindows)))) {
        $perlCmd = Get-Command perl -ErrorAction SilentlyContinue
        if ($perlCmd) {
            Write-Status "Found Perl: $($perlCmd.Source)" "Green"
            return $perlCmd.Source
        }
        Write-Status "WARNING: Perl not found." "Yellow"
        return $null
    }

    # Check for standalone Perl first (e.g. Strawberry Perl)
    $perlCandidates = Get-Command perl -ErrorAction SilentlyContinue -All
    foreach ($candidate in $perlCandidates) {
        if ($candidate.Source -notmatch '\\usr\\bin\\') {
            Write-Status "Found Perl: $($candidate.Source)" "Green"
            return $candidate.Source
        }
    }

    # Fall back to Git's bundled Perl
    $gitCmd = Get-Command git -ErrorAction SilentlyContinue
    if ($gitCmd) {
        $gitDir = Split-Path (Split-Path $gitCmd.Source)
        $gitPerl = Join-Path $gitDir "usr\bin\perl.exe"
        if (Test-Path $gitPerl) {
            Write-Status "Found Perl: $gitPerl (Git bundled)" "Green"
            return $gitPerl
        }
    }

    Write-Status "WARNING: Perl not found. OpenSSL build will fail." "Yellow"
    Write-Status "  Install Git for Windows (bundles Perl) or Strawberry Perl." "Yellow"
    return $null
}
