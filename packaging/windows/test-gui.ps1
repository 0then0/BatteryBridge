# Run only in a disposable Windows CI desktop. Preview never accesses IPC.
param([string]$Executable = 'target/debug/batterybridge-gui.exe')
$ErrorActionPreference = 'Stop'
$fixture = (Resolve-Path "$PSScriptRoot/../../apps/gui/tests/status.json").Path
$stderr = Join-Path $env:TEMP "batterybridge-gui-$PID-stderr.log"
$stdout = Join-Path $env:TEMP "batterybridge-gui-$PID-stdout.log"
$process = Start-Process -FilePath $Executable -ArgumentList @('--preview', "`"$fixture`"") -RedirectStandardError $stderr -RedirectStandardOutput $stdout -PassThru
try {
    if ($process.WaitForExit(8000)) {
        Get-Content $stderr,$stdout
        throw "Native GUI exited during startup: $($process.ExitCode)"
    }
    $process.Refresh()
    if ($process.MainWindowHandle -eq 0) {
        Get-Content $stderr,$stdout
        throw 'Native GUI did not create a window'
    }
} finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}
Write-Output 'Native GUI window startup: passed'
