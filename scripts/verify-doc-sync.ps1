# scripts/verify-doc-sync.ps1
# Verifies that host inspection topics and identity grounding are synchronized across documentation and source code.

$HostInspectPath = "src/tools/host_inspect.rs"
$Docs = @("README.md", "CAPABILITIES.md", "CLAUDE.md")
$IdentityDocs = @("README.md", "CLAUDE.md")
$Keywords = @("Senior SysAdmin", "Network Admin")

Write-Host "--- Hematite Documentation Sync Audit ---" -ForegroundColor Cyan

# 1. Extract topics from source
if (-not (Test-Path $HostInspectPath)) {
    Write-Error "Source file not found: $HostInspectPath"
    exit 1
}

$Content = Get-Content $HostInspectPath -Raw
# Parsing the "Unknown inspect_host topic" error message as the definitive list
if ($Content -match '"Unknown inspect_host topic ''.*?''. Use one of: (.*?)\."') {
    $TopicsList = $Matches[1]
    $Topics = $TopicsList.Split(",") | ForEach-Object { $_.Trim() }
    $ActualCount = $Topics.Count
    Write-Host "[SOURCE] Detected $ActualCount unique diagnostic topics in $HostInspectPath" -ForegroundColor Green
} else {
    Write-Error "Could not parse topic list from $HostInspectPath. Check the error message at the end of inspect_host()."
    exit 1
}

$HadError = $false

# 2. Check counts in Docs
foreach ($Doc in $Docs) {
    if (Test-Path $Doc) {
        $DocContent = Get-Content $Doc -Raw
        
        # Regex to find the primary total count first: "XX+ read-only inspection topics" or "covers XX+ topics"
        $FoundTotal = $false
        if ($DocContent -match "(\d+)\+ read-only inspection topics" -or $DocContent -match "covers (\d+)\+ topics" -or $DocContent -match "(\d+)\+ read-only diagnostic topics") {
            $DocCount = [int]$Matches[1]
            if ($DocCount -eq $ActualCount) {
                Write-Host "[DOC] ${Doc}: Total Match ($DocCount+)" -ForegroundColor Green
                $FoundTotal = $true
            } else {
                Write-Host "[DOC] ${Doc}: TOTAL MISMATCH! Source=$ActualCount, Doc=$DocCount" -ForegroundColor Red
                $HadError = $true
                $FoundTotal = $true
            }
        }
        
        # Check for SysAdmin category count (56+) if it's CAPABILITIES
        if ($DocContent -match "SysAdmin topics \((\d+)\+\)") {
            $SysAdminDocCount = [int]$Matches[1]
            $ExpectedSysAdmin = 56 # 76 - 12 (Network) - 8 (Dev)
            if ($SysAdminDocCount -eq $ExpectedSysAdmin) {
                 Write-Host "[DOC] ${Doc}: SysAdmin Category Match ($SysAdminDocCount+)" -ForegroundColor Green
            } else {
                 Write-Host "[DOC] ${Doc}: SysAdmin Category MISMATCH! Expected=$ExpectedSysAdmin, Doc=$SysAdminDocCount" -ForegroundColor Red
                 $HadError = $true
            }
        }

        if (-not $FoundTotal) {
             # If we didn't find a total but found a SysAdmin count, maybe it's partially synced
             Write-Host "[DOC] ${Doc}: Primary total count pattern not found." -ForegroundColor Yellow
        }
    } else {
        Write-Host "[DOC] ${Doc}: File not found (skipping)" -ForegroundColor Gray
    }
}

# 3. Check Identity Grounding
foreach ($Doc in $IdentityDocs) {
    if (Test-Path $Doc) {
        $DocContent = Get-Content $Doc -Raw
        foreach ($Keyword in $Keywords) {
            if ($DocContent -like "*$Keyword*") {
                Write-Host "[ID] ${Doc}: Found grounding keyword '$Keyword'" -ForegroundColor Green
            } else {
                Write-Host "[ID] ${Doc}: MISSING grounding keyword '$Keyword'" -ForegroundColor Red
                $HadError = $true
            }
        }
    }
}

if ($HadError) {
    Write-Host "`nFAILED: Documentation drift or identity regression detected." -ForegroundColor Red
    exit 1
} else {
    Write-Host "`nSUCCESS: All documentation is synchronized and grounded." -ForegroundColor Green
    exit 0
}
