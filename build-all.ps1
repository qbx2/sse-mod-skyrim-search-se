# Builds one DLL per supported game version and assembles a FOMOD-ready folder in ./dist
#
#   dist/fomod/...                                   installer
#   dist/<folder>/SKSE/Plugins/skyrim_search_se.dll  one per game version
#
# Zip the *contents* of ./dist to get an archive installable with MO2 / Vortex.

$ErrorActionPreference = 'Stop'

# game runtime version -> folder referenced by fomod/ModuleConfig.xml
$targets = [ordered]@{
    '1.6.1170.0' = 'AE-1.6.1170'
    '1.4.15.0'   = 'VR-1.4.15'
}

$dll = 'target\x86_64-pc-windows-gnu\release\skyrim_search_se.dll'

Push-Location $PSScriptRoot
$original = Get-Content 'target_version.txt' -Raw
try {
    if (Test-Path 'dist') { Remove-Item 'dist' -Recurse -Force }

    foreach ($version in $targets.Keys) {
        Write-Host "=== Building for $version ==="
        Set-Content -Path 'target_version.txt' -Value $version -NoNewline

        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed for $version" }

        $out = Join-Path 'dist' (Join-Path $targets[$version] 'SKSE\Plugins')
        New-Item -ItemType Directory -Force -Path $out | Out-Null
        Copy-Item $dll $out
    }

    Copy-Item 'fomod' (Join-Path 'dist' 'fomod') -Recurse

    # The mod version lives only in Cargo.toml; stamp it into the installer.
    $modVersion = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages[0].version
    $info = Join-Path 'dist' 'fomod\info.xml'
    $xml = Get-Content $info -Raw
    if ($xml -notmatch '<Version>@VERSION@</Version>') { throw "version placeholder not found in $info" }
    [IO.File]::WriteAllText((Resolve-Path $info), $xml.Replace('@VERSION@', $modVersion))

    Write-Host "Done: $(Join-Path $PSScriptRoot 'dist')"
}
finally {
    Set-Content -Path 'target_version.txt' -Value $original -NoNewline
    Pop-Location
}
