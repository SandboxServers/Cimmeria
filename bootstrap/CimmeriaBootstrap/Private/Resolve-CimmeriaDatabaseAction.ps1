function Resolve-CimmeriaDatabaseAction {
    <#
    .SYNOPSIS
        Picks the database action for a load from the flags and the state of sgw.
    .DESCRIPTION
        Pure routing, no SQL. Returns one of:
          Throw     -Force and -Wipe together (checked before any SQL runs)
          FullLoad  no schema: load database.sql from scratch
          Skip      schema present and fully seeded: nothing to do
          Refresh   resources refresh that keeps accounts and characters
          Replay    schema present, seed incomplete, no accounts: replay database.sql
          Wipe      -Wipe: the caller counts, backs up and drops, then full load
    .PARAMETER Reset
        -ResetDatabase. Does not change the routing; the caller prints counts first.
    #>
    param(
        [switch]$Force,
        [switch]$Wipe,
        [switch]$Reset,
        [switch]$SchemaExists,
        [switch]$SeedComplete,
        [switch]$HasAccounts
    )

    $null = $Reset
    if ($Force -and $Wipe) { return 'Throw' }
    if ($Wipe) { return 'Wipe' }
    if (-not $SchemaExists) { return 'FullLoad' }
    if ($Force) { return 'Refresh' }
    if ($SeedComplete) { return 'Skip' }
    if ($HasAccounts) { return 'Refresh' }
    return 'Replay'
}
