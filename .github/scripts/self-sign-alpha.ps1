# Self-sign NSIS/MSI under C:\t\release\bundle when repo secrets exist.
# Forks without VOCAWIN_SELF_SIGN_PFX / VOCAWIN_SELF_SIGN_PASSWORD skip
# and leave the installers unsigned. Never print the PFX, password, or key.

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

$expectedSha256 = "CF7E2104A2745DB25BC865330EEA3F73A1716AD69F49B67953D783D2097E916F"
$bundleRoot = "C:\t\release\bundle"
$pfxPath = Join-Path $env:RUNNER_TEMP "vocawin-alpha.pfx"
$cerPath = Join-Path $env:RUNNER_TEMP "vocawin-alpha.cer"
$importedThumb = $null

function Set-SignedOutput([string]$Value) {
    if ($env:GITHUB_OUTPUT) {
        "signed=$Value" | Out-File -FilePath $env:GITHUB_OUTPUT -Append -Encoding utf8
    }
}

function Get-CertSha256([System.Security.Cryptography.X509Certificates.X509Certificate2]$Cert) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($sha.ComputeHash($Cert.RawData))).Replace("-", "")
    } finally {
        $sha.Dispose()
    }
}

function Write-PublicCer([System.Security.Cryptography.X509Certificates.X509Certificate2]$Cert, [string]$Path) {
    $b64 = [Convert]::ToBase64String($Cert.RawData)
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("-----BEGIN CERTIFICATE-----") | Out-Null
    for ($i = 0; $i -lt $b64.Length; $i += 64) {
        $len = [Math]::Min(64, $b64.Length - $i)
        $lines.Add($b64.Substring($i, $len)) | Out-Null
    }
    $lines.Add("-----END CERTIFICATE-----") | Out-Null
    $utf8 = New-Object System.Text.UTF8Encoding $false
    [IO.File]::WriteAllLines($Path, $lines, $utf8)
}

function Get-SignToolPath {
    $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }
    $found = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
        Where-Object { $_.DirectoryName -match '\\x64$' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $found) {
        throw "signtool.exe not found. Windows SDK should be on windows-latest."
    }
    return $found.FullName
}

function Invoke-SignFile([string]$Tool, [string]$Thumb, [string]$FilePath, [string]$PfxFile) {
    $urls = @(
        "http://timestamp.digicert.com",
        "http://timestamp.sectigo.com"
    )
    foreach ($url in $urls) {
        Write-Host "Signing $FilePath"
        & $Tool sign /sha1 $Thumb /fd SHA256 /tr $url /td SHA256 $FilePath
        if ($LASTEXITCODE -eq 0) {
            return
        }
        Write-Host "Timestamp authority failed (exit $LASTEXITCODE). Trying the next public TSA."
    }
    if (Test-Path -LiteralPath $PfxFile) {
        foreach ($url in $urls) {
            Write-Host "Retrying $FilePath from the PFX"
            & $Tool sign /f $PfxFile /p $env:VOCAWIN_SELF_SIGN_PASSWORD /fd SHA256 /tr $url /td SHA256 $FilePath
            if ($LASTEXITCODE -eq 0) {
                return
            }
        }
    }
    throw "signtool sign failed for $FilePath"
}

if ([string]::IsNullOrWhiteSpace($env:VOCAWIN_SELF_SIGN_PFX) -or [string]::IsNullOrWhiteSpace($env:VOCAWIN_SELF_SIGN_PASSWORD)) {
    Write-Host "VOCAWIN_SELF_SIGN_* secrets are not set. Skipping Authenticode. Installers stay unsigned."
    Set-SignedOutput "false"
    exit 0
}

Write-Output "::add-mask::$($env:VOCAWIN_SELF_SIGN_PASSWORD)"

$files = @(Get-ChildItem -Path $bundleRoot -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like "*-setup.exe" -or $_.Extension -eq ".msi" } |
    Sort-Object FullName -Unique)

if ($files.Count -eq 0) {
    throw "No -setup.exe or .msi under $bundleRoot"
}

try {
    try {
        $pfxBytes = [Convert]::FromBase64String($env:VOCAWIN_SELF_SIGN_PFX.Trim())
    } catch {
        throw "VOCAWIN_SELF_SIGN_PFX is not valid base64."
    }
    [IO.File]::WriteAllBytes($pfxPath, $pfxBytes)

    $secure = ConvertTo-SecureString -String $env:VOCAWIN_SELF_SIGN_PASSWORD -AsPlainText -Force
    $imported = Import-PfxCertificate -FilePath $pfxPath -Password $secure -CertStoreLocation Cert:\CurrentUser\My
    if ($imported -is [System.Array]) {
        $imported = $imported | Where-Object { $_.HasPrivateKey } | Select-Object -First 1
    }
    if (-not $imported) {
        throw "Import-PfxCertificate did not return a certificate."
    }
    $importedThumb = $imported.Thumbprint

    $sha256 = Get-CertSha256 $imported
    if ($sha256 -ne $expectedSha256) {
        throw "PFX public cert SHA-256 does not match CN=VocaWin Alpha (self-signed)."
    }

    Write-PublicCer $imported $cerPath

    $repoCer = Join-Path $env:GITHUB_WORKSPACE "docs/certs/vocawin-alpha.cer"
    if (Test-Path $repoCer) {
        $repoCert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2 $repoCer
        $repoSha = Get-CertSha256 $repoCert
        if ($repoSha -ne $expectedSha256) {
            throw "docs/certs/vocawin-alpha.cer does not match the PFX public cert."
        }
        Write-Host "Repo public cert matches the PFX."
    }

    & certutil.exe -f -addstore -user TrustedPublisher $cerPath
    if ($LASTEXITCODE -ne 0) {
        throw "certutil could not import the public cert into Trusted Publishers."
    }
    # /pa walks the chain. A self-signed publisher also has to sit in Root on this runner.
    & certutil.exe -f -addstore -user Root $cerPath
    if ($LASTEXITCODE -ne 0) {
        throw "certutil could not import the public cert into Current User Root."
    }

    $signtool = Get-SignToolPath
    Write-Host "Using $signtool"

    foreach ($file in $files) {
        Invoke-SignFile $signtool $importedThumb $file.FullName $pfxPath
        & $signtool verify /pa $file.FullName
        if ($LASTEXITCODE -ne 0) {
            throw "signtool verify /pa failed for $($file.FullName)"
        }
        Write-Host "Verified $($file.FullName)"
    }

    Set-SignedOutput "true"
} finally {
    if (Test-Path -LiteralPath $pfxPath) {
        Remove-Item -LiteralPath $pfxPath -Force -ErrorAction SilentlyContinue
    }
    if ($importedThumb) {
        $storeItem = Join-Path "Cert:\CurrentUser\My" $importedThumb
        if (Test-Path $storeItem) {
            Remove-Item $storeItem -Force -ErrorAction SilentlyContinue
        }
    }
}
