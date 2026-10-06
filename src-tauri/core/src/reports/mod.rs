//! Report export (spec §51): JSON, CSV and self-contained HTML.

use crate::models::{CoreResult, ScanResult};
use crate::util::format_bytes_short;

pub fn export_json(result: &ScanResult, out_path: &str) -> CoreResult<()> {
    std::fs::write(out_path, serde_json::to_string_pretty(result)?)?;
    Ok(())
}

pub fn export_csv(result: &ScanResult, out_path: &str) -> CoreResult<()> {
    let mut csv = String::from("path,name,extension,size,allocated,category\n");
    for f in &result.large_files {
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",{},{},{}\n",
            f.path.replace('"', "\"\""),
            f.name.replace('"', "\"\""),
            f.extension,
            f.size,
            f.allocated,
            f.category.as_str()
        ));
    }
    std::fs::write(out_path, csv)?;
    Ok(())
}

pub fn export_html(result: &ScanResult, out_path: &str) -> CoreResult<()> {
    let s = &result.summary;
    let mut rows = String::new();
    for f in result.large_files.iter().take(100) {
        rows.push_str(&format!(
            "<tr><td>{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td></tr>",
            escape(&f.path),
            f.size,
            escape(&f.extension)
        ));
    }
    let mut ext_rows = String::new();
    for (ext, size, count) in result.extensions.iter().take(25) {
        ext_rows.push_str(&format!(
            "<tr><td>{}</td><td class=\"n\">{}</td><td class=\"n\">{count}</td></tr>",
            escape(ext),
            format_bytes_short(*size)
        ));
    }
    let mut cat_rows = String::new();
    for (cat, size, count) in &result.categories {
        cat_rows.push_str(&format!(
            "<tr><td>{:?}</td><td class=\"n\">{}</td><td class=\"n\">{count}</td></tr>",
            cat,
            format_bytes_short(*size)
        ));
    }
    let html = format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<title>SpaceForge report — {volume}</title>
<style>
body{{font-family:'Segoe UI',system-ui,sans-serif;background:#0b0e14;color:#e5e9f0;margin:0;padding:32px}}
h1{{font-size:22px;font-weight:600;margin:0 0 4px}} h2{{font-size:15px;margin:28px 0 8px;color:#8ab4ff}}
.kpis{{display:flex;gap:16px;margin:20px 0;flex-wrap:wrap}}
.kpi{{background:#151a26;border:1px solid #232b3d;border-radius:10px;padding:14px 20px;min-width:140px}}
.kpi b{{display:block;font-size:20px;margin-top:2px}}
table{{border-collapse:collapse;width:100%;font-size:13px}}
td,th{{padding:6px 10px;border-bottom:1px solid #1c2333;text-align:left}}
th{{color:#9aa4b8;font-weight:600}}
td.n{{text-align:right;font-variant-numeric:tabular-nums}}
.sub{{color:#8b93a7;font-size:12px}}
</style></head><body>
<h1>SpaceForge storage report</h1>
<div class="sub">Volume {volume} — generated {at} — {kind} scan, {dur} ms</div>
<div class="kpis">
<div class="kpi"><span class="sub">Total used</span><b>{used}</b></div>
<div class="kpi"><span class="sub">Total files</span><b>{files}</b></div>
<div class="kpi"><span class="sub">Directories</span><b>{dirs}</b></div>
<div class="kpi"><span class="sub">Unreadable entries</span><b>{unread}</b></div>
</div>
<h2>Top files (index threshold applied)</h2>
<table><tr><th>Path</th><th>Bytes</th><th>Ext</th></tr>{rows}</table>
<h2>Largest extensions</h2>
<table><tr><th>Extension</th><th>Size</th><th>Files</th></tr>{ext_rows}</table>
<h2>Categories</h2>
<table><tr><th>Category</th><th>Size</th><th>Files</th></tr>{cat_rows}</table>
</body></html>"#,
        volume = escape(&s.volume),
        at = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"),
        kind = format!("{:?}", s.kind),
        dur = s.duration_ms,
        used = format_bytes_short(s.volume_used),
        files = s.total_files,
        dirs = s.total_dirs,
        unread = s.unreadable_entries,
        rows = rows,
        ext_rows = ext_rows,
        cat_rows = cat_rows,
    );
    std::fs::write(out_path, html)?;
    Ok(())
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
