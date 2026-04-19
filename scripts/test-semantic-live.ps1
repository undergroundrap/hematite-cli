param([string]$Model = "bonsai-8b")
$b = "C:\Users\ocean\AntigravityProjects\Hematite-CLI\target\release\hematite.exe"
$i1 = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
$i2 = '{"jsonrpc":"2.0","method":"initialized"}'
$c  = '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"inspect_host","arguments":{"topic":"summary"}}}'
Write-Host "Running semantic-redact with model: $Model"
$raw = "$i1`n$i2`n$c" | & $b --mcp-server --semantic-redact --semantic-model $Model 2>$null
if ($raw -is [array]) { $raw = $raw -join "`n" }
$short = $raw.Substring(0, [Math]::Min(600, $raw.Length))
Write-Host "Response:"
Write-Host $short
$pass = 0
$fail = 0
if ($raw -match "semantic") { Write-Host "[PASS] semantic header"; $pass++ } else { Write-Host "[FAIL] semantic header missing"; $fail++ }
if ($raw -match "isError.*false") { Write-Host "[PASS] isError false"; $pass++ } else { Write-Host "[FAIL] isError not false"; $fail++ }
if ($raw -notmatch $env:USERNAME) { Write-Host "[PASS] username clean"; $pass++ } else { Write-Host "[FAIL] username leaked"; $fail++ }
if ($raw -notmatch $env:COMPUTERNAME) { Write-Host "[PASS] hostname clean"; $pass++ } else { Write-Host "[FAIL] hostname leaked"; $fail++ }
Write-Host ("Result: " + $pass + "/" + ($pass+$fail) + " passed")
if ($fail -gt 0) { exit 1 }
