/// Shared dark-theme HTML shell for all Hematite HTML outputs.
/// Any feature that saves an HTML file calls `build_html_shell` — consistent
/// look everywhere, CSS and JS defined in one place.

pub fn he(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&#34;")
}

const CSS: &str = r#":root{--bg:#000;--fg:#fff;--dim:#6b6b6b;--line:#1a1a1a;--line-2:#262626}
*{box-sizing:border-box;margin:0;padding:0}
html{scrollbar-width:thin;scrollbar-color:#2a2a2a #000}
::-webkit-scrollbar{width:8px}::-webkit-scrollbar-track{background:#000}::-webkit-scrollbar-thumb{background:#222;border-radius:999px;border:2px solid #000}::-webkit-scrollbar-thumb:hover{background:#333}
body{font-family:'Inter',-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;-webkit-font-smoothing:antialiased;-moz-osx-font-smoothing:grayscale;background:var(--bg);color:var(--fg);padding:2.5rem 1.5rem;min-height:100vh}
.wrap{max-width:900px;margin:0 auto}
header{background:#0a0a0a;border:1px solid var(--line-2);border-radius:18px;padding:2rem 2.25rem;margin-bottom:1rem}
h1{font-size:1.35rem;font-weight:600;letter-spacing:-0.025em;color:var(--fg);margin-bottom:.6rem}
.meta{font-size:.775rem;color:var(--dim);margin-bottom:1.25rem;display:flex;flex-wrap:wrap;gap:.4rem 1.5rem;letter-spacing:-0.005em}
.score-row{display:flex;align-items:center;gap:1rem;flex-wrap:wrap}
.grade{font-size:2rem;font-weight:800;width:3rem;height:3rem;border-radius:10px;display:flex;align-items:center;justify-content:center;flex-shrink:0;letter-spacing:-0.02em}
.gA{background:#14532d;color:#4ade80}.gB{background:#166534;color:#86efac}.gC{background:#78350f;color:#fbbf24}.gD{background:#7c2d12;color:#fb923c}.gF{background:#7f1d1d;color:#f87171}
.score-info h2{font-size:1rem;font-weight:600;letter-spacing:-0.02em;color:var(--fg)}.score-info p{color:#a3a3a3;font-size:.85rem;margin-top:.2rem;letter-spacing:-0.005em}
section{background:#0a0a0a;border:1px solid var(--line-2);border-radius:18px;padding:2rem 2.25rem;margin-bottom:1rem}
section>h2{font-size:.85rem;font-weight:600;letter-spacing:-0.01em;color:#d4d4d4;margin-bottom:1.25rem;padding-bottom:.75rem;border-bottom:1px solid var(--line)}
.recipe{padding:1rem 1.25rem;border-left:3px solid var(--line-2);border-radius:0 10px 10px 0;margin-bottom:.75rem;background:#111}
.recipe:last-child{margin-bottom:0}
.sev-action{border-left-color:#dc2626}.sev-investigate{border-left-color:#d97706}.sev-monitor{border-left-color:#3b82f6}
.recipe h3{font-size:.875rem;font-weight:600;letter-spacing:-0.015em;margin-bottom:.7rem;display:flex;align-items:center;gap:.5rem;flex-wrap:wrap;color:var(--fg)}
.badge{font-size:.65rem;font-weight:700;padding:.2rem .5rem;border-radius:5px;letter-spacing:.02em}
.b-action{background:#7f1d1d;color:#f87171}.b-investigate{background:#78350f;color:#fbbf24}.b-monitor{background:#1e3a5f;color:#93c5fd}
.recipe ol{padding-left:1.2rem;color:#d4d4d4}
.recipe li{margin-bottom:.4rem;line-height:1.6;font-size:.85rem}
.dig-deeper{font-size:.75rem;color:#4b4b4b;margin-top:.7rem}
.dig-deeper code{background:var(--line);padding:.1rem .3rem;border-radius:3px;font-size:.75rem;color:#6b6b6b}
.healthy{color:#4ade80;font-weight:500;font-size:.875rem;padding:.4rem 0;letter-spacing:-0.01em}
details{border:1px solid var(--line);border-radius:10px;margin-bottom:.6rem;overflow:hidden}
details:last-child{margin-bottom:0}
summary{cursor:pointer;font-weight:500;font-size:.8rem;color:#a3a3a3;padding:.7rem 1rem;background:#111;list-style:none;user-select:none;letter-spacing:-0.005em;transition:color 150ms ease,background 150ms ease}
summary::-webkit-details-marker{display:none}
summary::before{content:'▶  ';font-size:.6rem;color:var(--dim)}
details[open] summary::before{content:'▼  '}
summary:hover{background:#161616;color:var(--fg)}
pre{font-family:'Cascadia Code','JetBrains Mono','Fira Code',Consolas,monospace;font-size:.75rem;background:#000;color:#a3a3a3;padding:1.25rem;overflow-x:auto;white-space:pre-wrap;word-break:break-word;line-height:1.6;margin:0;border-top:1px solid var(--line)}
footer{text-align:center;color:var(--dim);font-size:.725rem;margin-top:1.5rem;padding-top:1rem;letter-spacing:-0.005em}
@media(max-width:640px){body{padding:1.5rem .75rem}header,section{padding:1.5rem;border-radius:14px}}
.copy-btn{display:inline-flex;align-items:center;gap:8px;margin-top:1.25rem;padding:9px 18px;border-radius:999px;font-family:inherit;font-size:.8rem;font-weight:500;letter-spacing:-0.005em;cursor:pointer;background:transparent;color:#d4d4d4;border:1px solid var(--line-2);transition:border-color 160ms ease,color 160ms ease,background 160ms ease}
.copy-btn:hover{border-color:var(--fg);color:var(--fg)}
.copy-btn.copied{border-color:#4ade80;color:#4ade80}
p{line-height:1.6;color:#d4d4d4;font-size:.9rem;letter-spacing:-0.005em}
p+p{margin-top:.75rem}
h2{font-size:1.1rem;font-weight:600;letter-spacing:-0.02em;margin-bottom:.75rem}
h3{font-size:.95rem;font-weight:600;letter-spacing:-0.015em;margin-bottom:.5rem}
ul,ol{padding-left:1.25rem;color:#d4d4d4}
li{margin-bottom:.4rem;line-height:1.6;font-size:.875rem}
a{color:#d4d4d4;text-decoration:none;border-bottom:1px solid var(--line-2);transition:border-color 150ms ease,color 150ms ease}
a:hover{color:var(--fg);border-bottom-color:var(--fg)}"#;

// DOM-driven — reads title, score, recipes, and sections from the page.
// Works for any content built with the shared CSS classes; no format args needed.
const COPY_SCRIPT: &str = r#"
function copyReport() {
  var btn = document.getElementById('copyBtn');
  if (!btn) return;
  var orig = btn.innerHTML;
  var lines = [];
  var h1 = document.querySelector('h1'); if (h1) lines.push(h1.innerText);
  var sh2 = document.querySelector('.score-info h2'); if (sh2) lines.push(sh2.innerText);
  var sp = document.querySelector('.score-info p'); if (sp) { lines.push(sp.innerText); lines.push(''); }
  document.querySelectorAll('.recipe').forEach(function(r) {
    var h = r.querySelector('h3'); if (h) lines.push(h.innerText);
    r.querySelectorAll('li').forEach(function(li) { lines.push('- ' + li.innerText); });
    lines.push('');
  });
  var dets = document.querySelectorAll('details');
  if (dets.length) {
    lines.push('--- Diagnostic Data ---');
    dets.forEach(function(d) {
      var s = d.querySelector('summary'); if (s) lines.push('\n[' + s.innerText.trim() + ']');
      var pre = d.querySelector('pre'); if (pre) lines.push(pre.innerText.trim());
    });
  } else {
    document.querySelectorAll('section').forEach(function(sec) {
      var sh = sec.querySelector('h2'); if (sh) lines.push('\n--- ' + sh.innerText + ' ---');
      lines.push(sec.innerText.replace(sh ? sh.innerText : '', '').trim());
    });
  }
  navigator.clipboard.writeText(lines.join('\n')).then(function() {
    btn.textContent = 'Copied!';
    btn.classList.add('copied');
    setTimeout(function() { btn.innerHTML = orig; btn.classList.remove('copied'); }, 2000);
  });
}
"#;

pub const COPY_BUTTON_HTML: &str = r#"<button class="copy-btn" id="copyBtn" onclick="copyReport()">
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
  Copy report for AI
</button>"#;

/// Wrap `content_html` in the Hematite dark-theme shell.
/// `content_html` is everything inside `.wrap` — assemble header cards,
/// sections, etc. in the caller. The shell provides CSS, JS, and the footer.
pub fn build_html_shell(title: &str, version: &str, content_html: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title}</title>
<style>{css}</style>
</head>
<body>
<div class="wrap">
{content}
</div>
<footer>Generated by Hematite v{version}</footer>
<script>{script}</script>
</body>
</html>"#,
        title = he(title),
        version = he(version),
        css = CSS,
        content = content_html,
        script = COPY_SCRIPT,
    )
}
