# Runs an executable with a short retry loop. Used by ctest to absorb
# transient Windows Defender real-time-scan locks that briefly cause
# CreateProcess to return ERROR_ACCESS_DENIED on newly-written binaries.
#
# Usage: pwsh -NoProfile -File scripts/run_with_retry.ps1 <exe-path> [args...]

param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$ExePath,
    [Parameter(ValueFromRemainingArguments = $true, Position = 1)]
    [string[]]$ExeArgs
)

$ErrorActionPreference = 'Continue'
$maxAttempts = 50
$delayMs = 100

for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
    $proc = Start-Process -FilePath $ExePath -ArgumentList $ExeArgs `
        -PassThru -Wait -NoNewWindow `
        -RedirectStandardOutput "$ExePath.stdout" `
        -RedirectStandardError  "$ExePath.stderr"
    if ($proc.ExitCode -eq 0) {
        $proc | Out-Null
        if (Test-Path "$ExePath.stdout") { Get-Content "$ExePath.stdout" }
        if (Test-Path "$ExePath.stderr") { Get-Content "$ExePath.stderr" }
        exit 0
    }
    # ERROR_ACCESS_DENIED (5) from the loader means a Defender scan
    # collision. Wait briefly and retry. Any other error is real.
    if ($proc.ExitCode -ne -1073741515 -and $proc.ExitCode -ne 5) {
        if (Test-Path "$ExePath.stdout") { Get-Content "$ExePath.stdout" }
        if (Test-Path "$ExePath.stderr") { Get-Content "$ExePath.stderr" }
        exit $proc.ExitCode
    }
    Start-Sleep -Milliseconds $delayMs
}

Write-Error "Failed to launch $ExePath after $maxAttempts attempts"
exit 1
