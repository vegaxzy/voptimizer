# =====================================================================
#  VOptimizer — Admin Tweak Self-Test  (fully reversible)
#
#  Runs the EXACT operations each admin-gated tweak performs:
#  capture-original -> apply -> verify -> ALWAYS restore (try/finally).
#  The restore runs even if a step errors mid-test, so the system is
#  always left exactly as it was found. Destructive (Debloat) tests use
#  synthetic files only — never your real data. Touches nothing outside
#  the tweaks we built (no Defender / UAC / BCD / services).
#
#  USAGE: right-click -> "Run with PowerShell" as administrator, OR:
#    powershell -ExecutionPolicy Bypass -File .\verify-admin-tweaks.ps1
# =====================================================================

$ErrorActionPreference = 'Stop'
$results = @()
function Note($name, $pass, $detail) {
    $script:results += [PSCustomObject]@{ Tweak = $name; Result = $(if ($pass) { 'PASS' } else { 'FAIL' }); Detail = $detail }
}

$elevated = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $elevated) {
    Write-Host "`n  This script must be run as ADMINISTRATOR. Right-click -> Run as admin.`n" -ForegroundColor Red
    exit 1
}
Write-Host "`n  Running VOptimizer admin self-test (elevated, fully reversible)...`n" -ForegroundColor Cyan

# 1) keep-kernel-in-ram : Memory Management\DisablePagingExecutive = 1
$p = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management'
$orig = (Get-ItemProperty $p -Name DisablePagingExecutive -EA SilentlyContinue).DisablePagingExecutive
try {
    Set-ItemProperty $p DisablePagingExecutive 1 -Type DWord
    $applied = (Get-ItemProperty $p -Name DisablePagingExecutive).DisablePagingExecutive
    Note 'keep-kernel-in-ram' ($applied -eq 1) "applied=$applied"
} catch { Note 'keep-kernel-in-ram' $false $_.Exception.Message }
finally { if ($null -eq $orig) { Remove-ItemProperty $p -Name DisablePagingExecutive -EA SilentlyContinue } else { Set-ItemProperty $p DisablePagingExecutive $orig -Type DWord } }

# 2) foreground-boost : PriorityControl\Win32PrioritySeparation = 0x1A
$p = 'HKLM:\SYSTEM\CurrentControlSet\Control\PriorityControl'
$orig = (Get-ItemProperty $p -Name Win32PrioritySeparation -EA SilentlyContinue).Win32PrioritySeparation
try {
    Set-ItemProperty $p Win32PrioritySeparation 0x1A -Type DWord
    $applied = (Get-ItemProperty $p -Name Win32PrioritySeparation).Win32PrioritySeparation
    Note 'foreground-boost' ($applied -eq 26) "applied=$applied (orig $orig)"
} catch { Note 'foreground-boost' $false $_.Exception.Message }
finally { if ($null -eq $orig) { Remove-ItemProperty $p -Name Win32PrioritySeparation -EA SilentlyContinue } else { Set-ItemProperty $p Win32PrioritySeparation $orig -Type DWord } }

# 3) disable-nic-power-saving : physical adapter PnPCapabilities = 24
$net = 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e972-e325-11ce-bfc1-08002be10318}'
$adapter = Get-ChildItem $net | Where-Object { $_.PSChildName -match '^\d{4}$' } | ForEach-Object {
    $pr = Get-ItemProperty $_.PSPath -EA SilentlyContinue
    if ($pr.NetCfgInstanceId -and ("$($pr.ComponentId)".ToLower().StartsWith('pci\') -or "$($pr.ComponentId)".ToLower().StartsWith('usb\'))) { $_.PSPath }
} | Select-Object -First 1
if ($adapter) {
    $orig = (Get-ItemProperty $adapter -Name PnPCapabilities -EA SilentlyContinue).PnPCapabilities
    try {
        Set-ItemProperty $adapter PnPCapabilities 24 -Type DWord
        $applied = (Get-ItemProperty $adapter -Name PnPCapabilities).PnPCapabilities
        Note 'disable-nic-power-saving' ($applied -eq 24) "applied=$applied (orig $(if($null -eq $orig){'<none>'}else{$orig}))"
    } catch { Note 'disable-nic-power-saving' $false $_.Exception.Message }
    finally { if ($null -eq $orig) { Remove-ItemProperty $adapter -Name PnPCapabilities -EA SilentlyContinue } else { Set-ItemProperty $adapter PnPCapabilities $orig -Type DWord } }
} else { Note 'disable-nic-power-saving' $false 'no physical adapter found' }

# 4) set-game-priority (IFEO) : PerfOptions\CpuPriorityClass = 3 for a test exe
$exe = 'voptverify_game.exe'
$base = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\$exe"
$preexisted = Test-Path $base
try {
    New-Item "$base\PerfOptions" -Force | Out-Null
    Set-ItemProperty "$base\PerfOptions" CpuPriorityClass 3 -Type DWord
    Set-ItemProperty "$base\PerfOptions" IoPriority 3 -Type DWord
    Set-ItemProperty "$base\PerfOptions" PagePriority 5 -Type DWord
    $applied = (Get-ItemProperty "$base\PerfOptions" -Name CpuPriorityClass).CpuPriorityClass
    Note 'set-game-priority' ($applied -eq 3) "applied=$applied"
} catch { Note 'set-game-priority' $false $_.Exception.Message }
finally { if (-not $preexisted) { Remove-Item $base -Recurse -Force -EA SilentlyContinue } }   # only remove what we created

# 5) disable-hibernate : powercfg /hibernate off  (state captured + restored)
$rp = 'HKLM:\SYSTEM\CurrentControlSet\Control\Power'
$orig = (Get-ItemProperty $rp -Name HibernateEnabled -EA SilentlyContinue).HibernateEnabled
try {
    powercfg /hibernate off | Out-Null
    $applied = (Get-ItemProperty $rp -Name HibernateEnabled).HibernateEnabled
    Note 'disable-hibernate' ($applied -eq 0) "applied=$applied (orig $orig)"
} catch { Note 'disable-hibernate' $false $_.Exception.Message }
finally { if ($orig -eq 1) { powercfg /hibernate on | Out-Null } }   # restore ON only if it was ON

# 6) disable-lso : property-agnostic (V1/V2 adapters), per-adapter capture + restore
function Get-LsoFlag($adapter, $family) {
    foreach ($prop in "IPv${family}Enabled", "V2IPv${family}Enabled", "V1IPv${family}Enabled") {
        if ($null -ne $adapter.$prop) { return [bool]$adapter.$prop }
    }
    return $null
}
$lso = @(Get-NetAdapterLso -EA SilentlyContinue)
if ($lso.Count -eq 0) {
    Note 'disable-lso' $true 'no LSO-capable adapters present (nothing to do)'
} else {
    $lsoOrig = @{}
    foreach ($a in $lso) { $lsoOrig[$a.Name] = @((Get-LsoFlag $a 4), (Get-LsoFlag $a 6)) }
    $first = $lso[0].Name
    try {
        Disable-NetAdapterLso -Name '*' -EA SilentlyContinue
        $applied = Get-LsoFlag (Get-NetAdapterLso -Name $first -EA SilentlyContinue) 4
        Note 'disable-lso' ($applied -eq $false) "IPv4 LSO now=$applied on '$first'"
    } catch { Note 'disable-lso' $false $_.Exception.Message }
    finally {
        foreach ($k in $lsoOrig.Keys) {
            if ($lsoOrig[$k][0]) { Enable-NetAdapterLso -Name $k -IPv4 -EA SilentlyContinue } else { Disable-NetAdapterLso -Name $k -IPv4 -EA SilentlyContinue }
            if ($null -ne $lsoOrig[$k][1]) { if ($lsoOrig[$k][1]) { Enable-NetAdapterLso -Name $k -IPv6 -EA SilentlyContinue } else { Disable-NetAdapterLso -Name $k -IPv6 -EA SilentlyContinue } }
        }
    }
}

# 7) Debloat admin clean : synthetic file in Windows\Temp only (no real data)
$wtmp = "$env:SystemRoot\Temp\voptverify_test"
try {
    New-Item -ItemType Directory $wtmp -Force | Out-Null
    [IO.File]::WriteAllBytes("$wtmp\junk.bin", (New-Object byte[] (1MB)))
    Remove-Item $wtmp -Recurse -Force
    Note 'debloat windows-temp clean' (-not (Test-Path $wtmp)) 'wrote + cleaned 1MB synthetic file (admin)'
} catch { Note 'debloat windows-temp clean' $false $_.Exception.Message }
finally { if (Test-Path $wtmp) { Remove-Item $wtmp -Recurse -Force -EA SilentlyContinue } }

Write-Host ""
$results | Format-Table -AutoSize
$fail = ($results | Where-Object Result -eq 'FAIL').Count
if ($fail -eq 0) { Write-Host "  ALL $($results.Count) ADMIN OPERATIONS PASSED — system restored to original state.`n" -ForegroundColor Green }
else { Write-Host "  $fail FAILED — review the Detail column above.`n" -ForegroundColor Red }
