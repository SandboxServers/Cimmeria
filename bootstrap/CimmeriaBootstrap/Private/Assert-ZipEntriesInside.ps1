function Assert-ZipEntriesInside {
    <#
    .SYNOPSIS
        Throws unless every entry of a zip resolves to a path inside a folder.
        Run before extracting: it rejects `..\` entries and absolute paths.
    #>
    param(
        [Parameter(Mandatory)][string]$ZipPath,
        [Parameter(Mandatory)][string]$Destination
    )

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $root = [IO.Path]::GetFullPath($Destination).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    $zip = [IO.Compression.ZipFile]::OpenRead($ZipPath)
    try {
        foreach ($entry in $zip.Entries) {
            $target = [IO.Path]::GetFullPath([IO.Path]::Combine($root, $entry.FullName))
            if (-not $target.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) {
                throw "$(Split-Path -Leaf $ZipPath) has an entry outside the folder it unpacks into ('$($entry.FullName)'). Not installed."
            }
        }
    } finally {
        $zip.Dispose()
    }
}
