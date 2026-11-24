# Versión Windows de medir.sh (PowerShell 7). Mide Flutter release vs GPUI
# release en la pantalla «Instancias» con los mismos datos sembrados.
#
#   pwsh tools/medir.ps1 -Flutter app\build\windows\x64\runner\Release `
#        -Gpui ui-gpui\target\release\shard-gpui.exe [-Runs 3] [-Out medicion-win]
#
# Por repetición: arranque hasta el primer fotograma (marcador
# SHARD_FIRST_FRAME en stdout), memoria a los 10 s (Working Set = RSS y
# Private Bytes; Windows no tiene PSS), CPU en reposo entre los segundos 10 y
# 20 (% de un núcleo). La fase de uso de 30 s y la prueba de bandeja se hacen
# a mano (ver ENTREGA-SPIKE.md): Windows no tiene xdotool ni D-Bus.
param(
  [Parameter(Mandatory)] [string] $Flutter,
  [Parameter(Mandatory)] [string] $Gpui,
  [int] $Runs = 3,
  [string] $Out = "medicion-win-$(Get-Date -Format yyyyMMdd-HHmmss)",
  [string[]] $Modes = @('anim', 'quieto')
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force $Out | Out-Null
$rows = @()

function Seed($dir, $mode) {
  & $Gpui --seed $dir | Out-Null
  if ($mode -eq 'quieto') { Set-Content -Path (Join-Path $dir 'ui.json') -Value '{"firstRunDone":true,"performance":true}' }
}

foreach ($mode in $Modes) {
  foreach ($run in 1..$Runs) {
    foreach ($app in 'flutter', 'gpui') {
      $data = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid())
      Seed $data $mode
      $log = Join-Path $Out "$app-$mode-$run.log"
      $psi = [Diagnostics.ProcessStartInfo]::new()
      $psi.RedirectStandardOutput = $true
      $psi.UseShellExecute = $false
      if ($app -eq 'flutter') {
        $psi.FileName = Join-Path $Flutter 'shard.exe'
        $psi.Environment['SHARD_DATA_DIR'] = $data
        $psi.Environment['SHARD_START_ROUTE'] = '/instances'
        $psi.Environment['SHARD_SPIKE_TIMING'] = '1'
      } else {
        $psi.FileName = $Gpui
        $psi.ArgumentList.Add('--data-dir'); $psi.ArgumentList.Add($data)
      }
      $t0 = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
      $p = [Diagnostics.Process]::Start($psi)
      $ff = $null
      $sw = [Diagnostics.Stopwatch]::StartNew()
      while (-not $ff -and $sw.Elapsed.TotalSeconds -lt 30) {
        $line = $p.StandardOutput.ReadLine()
        if ($null -eq $line) { break }
        Add-Content $log $line
        if ($line -match 'SHARD_FIRST_FRAME unix_ms=(\d+)') { $ff = [int64]$Matches[1] - $t0 }
      }
      $rest = 10000 - ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - $t0)
      if ($rest -gt 0) { Start-Sleep -Milliseconds $rest }
      $p.Refresh()
      $ws10 = $p.WorkingSet64; $priv10 = $p.PrivateMemorySize64; $c0 = $p.TotalProcessorTime
      Start-Sleep -Seconds 10
      $p.Refresh()
      $cpu = [math]::Round(($p.TotalProcessorTime - $c0).TotalMilliseconds / 10000 * 100, 2)
      $rows += [pscustomobject]@{ app = $app; modo = $mode; run = $run; arranque_ms = $ff
        ws10_mb = [math]::Round($ws10 / 1MB, 1); privados10_mb = [math]::Round($priv10 / 1MB, 1); cpu_reposo_pct = $cpu }
      $rows[-1] | Format-Table -HideTableHeaders | Out-String | Write-Host
      Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
      Remove-Item -Recurse -Force $data -ErrorAction SilentlyContinue
      Start-Sleep -Seconds 2
    }
  }
}
$rows | Export-Csv -NoTypeInformation (Join-Path $Out 'runs.csv')

$md = @('# Resumen Windows (medianas)', '')
foreach ($mode in $Modes) {
  $md += "## Modo $mode", '', '| métrica | Flutter | GPUI | GPUI/Flutter |', '|---|---|---|---|'
  foreach ($k in 'arranque_ms', 'ws10_mb', 'privados10_mb', 'cpu_reposo_pct') {
    $med = @{}
    foreach ($a in 'flutter', 'gpui') {
      $v = @($rows | Where-Object { $_.app -eq $a -and $_.modo -eq $mode -and $null -ne $_.$k } | ForEach-Object { [double]$_.$k } | Sort-Object)
      $med[$a] = if ($v.Count) { $v[[int][math]::Floor(($v.Count - 1) / 2)] } else { $null }
    }
    $ratio = if ($med.flutter) { '{0:N0} %' -f ($med.gpui / $med.flutter * 100) } else { '—' }
    $md += "| $k | $($med.flutter) | $($med.gpui) | $ratio |"
  }
  $md += ''
}
$fl = (Get-ChildItem -Recurse -File $Flutter | Measure-Object Length -Sum).Sum
$md += '## Tamaño', '', "| Flutter (carpeta Release) | $([math]::Round($fl / 1MB, 1)) MB |", "| GPUI (exe) | $([math]::Round((Get-Item $Gpui).Length / 1MB, 1)) MB |"
$md | Set-Content (Join-Path $Out 'resumen.md')
$md -join "`n"
