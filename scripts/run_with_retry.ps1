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

$ErrorActionPreference = 'Stop'
$maxAttempts = 50
$delayMs = 100
$stdoutPath = "$ExePath.stdout"
$stderrPath = "$ExePath.stderr"

function Invoke-Once {
    # Start-Process rejects -ArgumentList $null / empty on modern PowerShell.
    $startParams = @{
        FilePath               = $ExePath
        PassThru               = $true
        Wait                   = $true
        NoNewWindow            = $true
        RedirectStandardOutput = $stdoutPath
        RedirectStandardError  = $stderrPath
    }
    if ($null -ne $ExeArgs -and $ExeArgs.Count -gt 0) {
        $startParams['ArgumentList'] = $ExeArgs
    }
    return Start-Process @startParams
}

for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
    try {
        $proc = Invoke-Once
    } catch {
        # Treat launch exceptions (including Defender locks) as retryable
        # for a few attempts, then fail hard.
        if ($attempt -eq $maxAttempts) {
            Write-Error "Failed to launch $ExePath : $_"
            exit 1
        }
        Start-Sleep -Milliseconds $delayMs
        continue
    }

    if ($null -eq $proc) {
        Write-Error "Start-Process returned null for $ExePath"
        exit 1
    }

    $code = $proc.ExitCode
    if ($code -eq 0) {
        if (Test-Path $stdoutPath) { Get-Content $stdoutPath }
        if (Test-Path $stderrPath) { Get-Content $stderrPath }
        exit 0
    }

    # ERROR_ACCESS_DENIED (5) / STATUS_DLL_INIT_FAILED-ish from Defender
    # collisions. Retry briefly; any other code is a real test failure.
    if ($code -ne -1073741515 -and $code -ne 5) {
        if (Test-Path $stdoutPath) { Get-Content $stdoutPath }
        if (Test-Path $stderrPath) { Get-Content $stderrPath }
        exit $code
    }
    Start-Sleep -Milliseconds $delayMs
}

Write-Error "Failed to launch $ExePath after $maxAttempts attempts"
exit 1
