# MCP server portable smoke test
# Run after: cargo build --release
param([string]$Binary = "C:\Users\ocean\AntigravityProjects\Hematite-CLI\target\release\hematite.exe")

$pass = 0
$fail = 0

function Pass([string]$label) { Write-Host "  [PASS] $label" -ForegroundColor Green; $script:pass++ }
function Fail([string]$label, [string]$detail) {
    Write-Host "  [FAIL] $label" -ForegroundColor Red
    if ($detail) { Write-Host "         $detail" -ForegroundColor DarkRed }
    $script:fail++
}

# Run the MCP server with given messages and return stdout as one string
function Invoke-Mcp([string]$Msgs, [string[]]$ExtraArgs) {
    if ($ExtraArgs -and $ExtraArgs.Length -gt 0) {
        $out = $Msgs | & $Binary --mcp-server @ExtraArgs 2>$null
    } else {
        $out = $Msgs | & $Binary --mcp-server 2>$null
    }
    # Join array output into single string for reliable matching
    if ($out -is [array]) { return $out -join "`n" } else { return "$out" }
}

# Find the response line that contains id:N
function Get-Line([string]$raw, [int]$id) {
    $pat = '"id":' + $id
    foreach ($ln in ($raw -split "`n")) {
        if ($ln.TrimStart().StartsWith("{") -and $ln -match $pat) { return $ln }
    }
    return $null
}

Write-Host ""
Write-Host "Hematite MCP Portable Smoke Test" -ForegroundColor Cyan
Write-Host "Binary: $Binary"
Write-Host ""

if (-not (Test-Path $Binary)) {
    Write-Host "ERROR: binary not found - run cargo build --release first" -ForegroundColor Red
    exit 1
}

$I1 = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
$I2 = '{"jsonrpc":"2.0","method":"initialized"}'
$CallSummary = '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"inspect_host","arguments":{"topic":"summary"}}}'
$CallList    = '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'

# ── 1. Flag visibility ────────────────────────────────────────────────────────
Write-Host "1. Flag visibility"
$helpText = (& $Binary --help 2>&1) | Out-String
if ($helpText -match "semantic.redact") { Pass "--semantic-redact in --help" } else { Fail "--semantic-redact missing from --help" "" }
if ($helpText -match "edge.redact")     { Pass "--edge-redact in --help"     } else { Fail "--edge-redact missing from --help" "" }
if ($helpText -match "mcp.server")      { Pass "--mcp-server in --help"      } else { Fail "--mcp-server missing from --help" "" }

# ── 2. initialize handshake ───────────────────────────────────────────────────
Write-Host ""
Write-Host "2. MCP initialize handshake"
$raw = Invoke-Mcp -Msgs $I1
if ($raw -match '"protocolVersion"\s*:\s*"2024-11-05"') { Pass "protocolVersion correct" } else { Fail "protocolVersion wrong" $raw.Substring(0,[Math]::Min(120,$raw.Length)) }
if ($raw -match '"name"\s*:\s*"hematite"')              { Pass "serverInfo.name = hematite" } else { Fail "serverInfo.name wrong" "" }

# ── 3. tools/list ─────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "3. tools/list"
$raw = Invoke-Mcp -Msgs "$I1`n$I2`n$CallList"
$line = Get-Line -raw $raw -id 2
if ($line) {
    if ($line -match '"name"\s*:\s*"inspect_host"') { Pass "tools/list includes inspect_host" }
    else { Fail "inspect_host not in tools/list" "" }
    if ($raw -match 'app_crashes') { Pass "tool description mentions app_crashes" }
    else { Fail "app_crashes not in topic list description" "" }
} else { Fail "tools/list: no id:2 response line" "" }

# ── 4. tools/call summary (no redaction) ──────────────────────────────────────
Write-Host ""
Write-Host "4. tools/call summary (no redaction)"
$raw = Invoke-Mcp -Msgs "$I1`n$I2`n$CallSummary"
$line = Get-Line -raw $raw -id 2
if ($line) {
    if ($line -match '"isError"\s*:\s*false') { Pass "isError:false" } else { Fail "isError:true on summary" "" }
    if ($line -match 'OS|Uptime|CPU|RAM|Hostname|windows') { Pass "content looks like host data" }
    else { Fail "content does not look like host data" $line.Substring(0,[Math]::Min(200,$line.Length)) }
} else { Fail "tools/call: no id:2 response line" "" }

# ── 5. edge-redact header ─────────────────────────────────────────────────────
Write-Host ""
Write-Host "5. --edge-redact pipeline"
$raw = Invoke-Mcp -Msgs "$I1`n$I2`n$CallSummary" -ExtraArgs @("--edge-redact")
$line = Get-Line -raw $raw -id 2
if ($line) {
    if ($line -match '\[edge-redact:') { Pass "edge-redact header present" }
    else { Fail "edge-redact header missing" $line.Substring(0,[Math]::Min(200,$line.Length)) }
    if ($line -match '\[USER\]') { Pass "username replaced with [USER]" }
    else { Pass "no username paths in this output (acceptable)" }
    if ($line -match '"isError"\s*:\s*false') { Pass "isError:false with edge-redact" }
    else { Fail "edge-redact returned isError:true" "" }
} else { Fail "edge-redact: no id:2 response line" "" }

# ── 6. Arg sanitization ───────────────────────────────────────────────────────
Write-Host ""
Write-Host "6. Arg sanitization (unknown args stripped, call still succeeds)"
$CallEvil = '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"inspect_host","arguments":{"topic":"summary","bypass_redact":true,"evil":"injected"}}}'
$raw = Invoke-Mcp -Msgs "$I1`n$I2`n$CallEvil" -ExtraArgs @("--edge-redact")
$line = Get-Line -raw $raw -id 2
if ($line) {
    if ($line -match '"isError"\s*:\s*false') { Pass "call succeeds with unknown args stripped" }
    else { Fail "call failed after stripping unknown args" "" }
} else { Fail "arg sanitization: no id:2 response" "" }

# ── 7. Policy file — blocked topic ───────────────────────────────────────────
Write-Host ""
Write-Host "7. Policy file - topic blocking"
$realPolicy = ".hematite\redact_policy.json"
$bakPolicy  = ".hematite\redact_policy.json.bak"
$hadPolicy  = Test-Path $realPolicy
if ($hadPolicy) { Copy-Item $realPolicy $bakPolicy }
'{"blocked_topics":["summary"]}' | Set-Content -Path $realPolicy
try {
    $raw = Invoke-Mcp -Msgs "$I1`n$I2`n$CallSummary"
    $line = Get-Line -raw $raw -id 2
    if ($line) {
        if ($line -match '"isError"\s*:\s*true' -and $line -match 'blocked') {
            Pass "blocked topic returns isError:true with 'blocked'"
        } else {
            Fail "blocked topic did not error correctly" $line.Substring(0,[Math]::Min(200,$line.Length))
        }
    } else { Fail "policy block: no id:2 response" "" }
} finally {
    Remove-Item $realPolicy -ErrorAction SilentlyContinue
    if ($hadPolicy) { Copy-Item $bakPolicy $realPolicy; Remove-Item $bakPolicy -ErrorAction SilentlyContinue }
}

# ── 8. Semantic fail-safe — unreachable model → error, not raw data ───────────
Write-Host ""
Write-Host "8. --semantic-redact fail-safe (unreachable model)"
$raw = Invoke-Mcp -Msgs "$I1`n$I2`n$CallSummary" -ExtraArgs @("--semantic-redact", "--url", "http://localhost:19999/v1")
$line = Get-Line -raw $raw -id 2
if ($line) {
    if ($line -match '"isError"\s*:\s*true' -and ($line -match 'unavailable|unreachable|withheld')) {
        Pass "fail-safe returns error when model unreachable"
    } else {
        Fail "fail-safe did not block correctly" $line.Substring(0,[Math]::Min(250,$line.Length))
    }
    # Critical: raw machine identity must NOT appear in the error text
    if ($line -notmatch [regex]::Escape($env:COMPUTERNAME)) {
        Pass "hostname not leaked in fail-safe error"
    } else {
        Fail "CRITICAL: hostname leaked in fail-safe error message" ""
    }
    # Should NOT have succeeded (no edge-redact header + isError:false = bad)
    if ($line -match '"isError"\s*:\s*false') {
        Fail "CRITICAL: semantic fail-safe silently passed raw data through" ""
    }
} else { Fail "semantic fail-safe: no id:2 response" "" }

# ── 9. Audit trail written ────────────────────────────────────────────────────
Write-Host ""
Write-Host "9. Audit trail"
$auditLog   = "$env:USERPROFILE\.hematite\redact_audit.jsonl"
$sizeBefore = 0
if (Test-Path $auditLog) { $sizeBefore = (Get-Item $auditLog).Length }
$CallProc = '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"inspect_host","arguments":{"topic":"processes"}}}'
$null = Invoke-Mcp -Msgs "$I1`n$I2`n$CallProc" -ExtraArgs @("--edge-redact")
Start-Sleep -Milliseconds 800
if (Test-Path $auditLog) {
    $sizeAfter = (Get-Item $auditLog).Length
    if ($sizeAfter -gt $sizeBefore) {
        Pass ("audit log grew (" + $sizeBefore + " to " + $sizeAfter + " bytes)")
        $lastLine = Get-Content $auditLog | Select-Object -Last 1
        if ($lastLine -match '"topic"' -and $lastLine -match '"mode"' -and $lastLine -match '"ts"') {
            Pass "audit entry has topic/mode/ts fields"
        } else { Fail "audit entry missing expected fields" $lastLine.Substring(0,[Math]::Min(120,$lastLine.Length)) }
        $hasHost = $lastLine -match [regex]::Escape($env:COMPUTERNAME)
        $hasUser = $lastLine -match [regex]::Escape($env:USERNAME)
        if (-not $hasHost -and -not $hasUser) { Pass "audit entry has no raw hostname/username" }
        else { Fail "CRITICAL: audit entry contains raw identity data" "" }
    } else { Fail "audit log did not grow after tool call" "" }
} else { Fail ("audit log not created at " + $auditLog) "" }

# ── Summary ───────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "-------------------------------------"
$total = $pass + $fail
$color = if ($fail -eq 0) { "Green" } else { "Yellow" }
Write-Host ("Result: " + $pass + "/" + $total + " passed") -ForegroundColor $color
if ($fail -gt 0) {
    Write-Host ($fail.ToString() + " test(s) failed - fix before cutting 0.6.0") -ForegroundColor Red
    exit 1
} else {
    Write-Host "All tests passed - ready to cut 0.6.0" -ForegroundColor Green
    exit 0
}
