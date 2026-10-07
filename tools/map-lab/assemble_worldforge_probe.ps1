param(
    [Parameter(Mandatory=$true)][string]$ClientRoot,
    [Parameter(Mandatory=$true)][string]$InputMap,
    [Parameter(Mandatory=$true)][string]$OutputMap,
    [Parameter(Mandatory=$true)][string]$PreviousProbe,
    [string]$Patcher = 'B:\targets\Cimmeria\debug\upk_patch.exe'
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $InputMap -PathType Leaf)) { throw "Missing input map: $InputMap" }
if (-not (Test-Path -LiteralPath $PreviousProbe -PathType Leaf)) { throw "Missing previous probe: $PreviousProbe" }
if (Test-Path -LiteralPath $OutputMap) { throw "Output already exists: $OutputMap" }
if (-not (Test-Path -LiteralPath $Patcher -PathType Leaf)) { throw "Missing upk_patch: $Patcher" }

$agnos = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\Agnos\Agnos-00000005.umap'
$shell = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\Agnos\Agnos-0002fff9.umap'
$library = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\Agnos_Library\Agnos_Library-00000000.umap'
$sgc = Join-Path $ClientRoot 'SGWGame\CookedPC\Maps\SGC\SGC-00000002.umap'
$steps = @(
    @{ name='sky'; source=$library; root=307; at='0,2200,-13814' },
    @{ name='archive-elbow'; source=$shell; root=290; at='-2048,2220,-128'; yaw=0 },
    @{ name='index-entry-fourway'; source=$shell; root=265; at='0,2220,-128'; yaw=0 },
    @{ name='fabrication-elbow'; source=$shell; root=290; at='2048,2220,-128'; yaw=90 },
    @{ name='west-gallery-threeway'; source=$shell; root=375; at='-2048,4268,-128'; yaw=0 },
    @{ name='index-fourway'; source=$shell; root=265; at='0,4268,-128'; yaw=0 },
    @{ name='east-encounter-threeway'; source=$shell; root=375; at='2048,4268,-128'; yaw=180 },
    @{ name='north-west-gallery-elbow'; source=$shell; root=290; at='-2048,6316,-128'; yaw=270 },
    @{ name='north-observation-threeway'; source=$shell; root=375; at='0,6316,-128'; yaw=270 },
    @{ name='north-east-encounter-elbow'; source=$shell; root=290; at='2048,6316,-128'; yaw=180 },
    @{ name='south-garden-slab'; source=$sgc; root=1132; at='1205,-1898,-128' },
    @{ name='archive-shelf-a'; source=$agnos; root=156; at='-4500,2660,-128' },
    @{ name='archive-shelf-b'; source=$agnos; root=314; at='-3700,2660,-128' },
    @{ name='archive-stasis'; source=$agnos; root=145; at='-4150,1940,-128' },
    @{ name='fabrication-mainframe'; source=$agnos; root=209; at='4550,2560,-128' },
    @{ name='fabrication-console'; source=$agnos; root=275; at='3650,1930,-128' },
    @{ name='fabrication-bench'; source=$agnos; root=161; at='4300,1830,-128' },
    @{ name='west-link-terminal'; source=$agnos; root=275; at='-2520,4680,-128' },
    @{ name='east-link-bench'; source=$agnos; root=161; at='2520,4650,-128' },
    @{ name='north-gallery-stasis'; source=$agnos; root=145; at='-2430,6580,-128' },
    @{ name='north-gallery-shelves'; source=$agnos; root=273; at='2430,6590,-128' },
    @{ name='observation-terminal'; source=$agnos; root=301; at='0,6700,-128' },
    @{ name='garden-bench'; source=$agnos; root=161; at='350,-1740,-128' },
    @{ name='power-generator'; source=$PreviousProbe; root=111; at='500,4650,-130' },
    @{ name='gallery-cover-west-a'; source=$PreviousProbe; root=113; at='-2530,4130,-128' },
    @{ name='gallery-cover-west-b'; source=$PreviousProbe; root=113; at='-1550,4660,-128' },
    @{ name='gallery-cover-north-a'; source=$PreviousProbe; root=113; at='-2520,6100,-128' },
    @{ name='gallery-cover-north-b'; source=$PreviousProbe; root=113; at='-1580,6630,-128' },
    @{ name='encounter-cover-a'; source=$PreviousProbe; root=113; at='1560,4100,-128' },
    @{ name='encounter-cover-b'; source=$PreviousProbe; root=113; at='2540,6610,-128' }
)

$scratch = Join-Path ([IO.Path]::GetDirectoryName($OutputMap)) ([IO.Path]::GetFileNameWithoutExtension($OutputMap) + '.assembling.umap')
if (Test-Path -LiteralPath $scratch) { throw "Scratch output already exists: $scratch" }
$current = $InputMap
try {
    for ($i = 0; $i -lt $steps.Count; $i++) {
        $step = $steps[$i]
        $next = if ($i -eq $steps.Count - 1) { $OutputMap } else { "$scratch.$i" }
        Write-Host "[$($i + 1)/$($steps.Count)] $($step.name)"
        $argsForClone = @('clone-objects', $current, $step.source, $next, '--roots', $step.root, '--first-at', $step.at, '--strip-lightmaps')
        if ($step.ContainsKey('yaw')) { $argsForClone += @('--yaw-degrees', $step.yaw) }
        & $Patcher @argsForClone
        if ($LASTEXITCODE -ne 0) { throw "Clone failed: $($step.name)" }
        if ($current -like "$scratch.*") { Remove-Item -LiteralPath $current }
        $current = $next
    }
    & $Patcher audit-names $OutputMap
    if ($LASTEXITCODE -ne 0) { throw 'Property-name audit failed' }
} finally {
    if ($current -like "$scratch.*" -and (Test-Path -LiteralPath $current)) { Remove-Item -LiteralPath $current }
}
