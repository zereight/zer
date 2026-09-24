use serde::Serialize;

use crate::manifest::Manifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    Ok,
    OutOfHunk,
    UnknownFile,
    InvalidRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckResult {
    pub id: Option<String>,
    pub verdict: Verdict,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Summary {
    pub ok: usize,
    #[serde(rename = "out-of-hunk")]
    pub out_of_hunk: usize,
    #[serde(rename = "unknown-file")]
    pub unknown_file: usize,
    #[serde(rename = "invalid-record")]
    pub invalid_record: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerdictReport {
    pub results: Vec<CheckResult>,
    pub summary: Summary,
}

// Validate one raw record against the schema shape.
// Returns (id, file, line) only when every required field checks out.
fn as_finding(v: &serde_json::Value) -> Option<(String, String, u32)> {
    let id = v.get("id")?.as_str()?;
    let file = v.get("file")?.as_str()?;
    let line = v.get("line")?.as_u64()?;
    let severity = v.get("severity")?.as_str()?;
    let title = v.get("title")?.as_str()?;
    let evidence = v.get("evidence")?.as_str()?;
    let line = u32::try_from(line).ok()?;
    if id.is_empty() || file.is_empty() || line < 1 || title.is_empty() || evidence.is_empty() {
        return None;
    }
    if !matches!(severity, "critical" | "major" | "minor" | "trivial" | "info" | "refactor") {
        return None;
    }
    Some((id.to_string(), file.to_string(), line))
}

// One finding against one manifest. Pure.
pub fn check_finding(manifest: &Manifest, finding: &serde_json::Value) -> CheckResult {
    let id = finding.get("id").and_then(|v| v.as_str()).map(String::from);
    let Some((rid, file, line)) = as_finding(finding) else {
        return CheckResult {
            id,
            verdict: Verdict::InvalidRecord,
            reason: Some("record must match schema/finding.schema.json required fields".to_string()),
        };
    };
    let Some(entry) = manifest.files.iter().find(|f| f.path == file) else {
        return CheckResult {
            id,
            verdict: Verdict::UnknownFile,
            reason: Some(format!("file not in manifest: {file}")),
        };
    };
    let hit = entry.hunks.iter().any(|h| line >= h.start && line < h.start.saturating_add(h.count));
    if !hit {
        return CheckResult {
            id,
            verdict: Verdict::OutOfHunk,
            reason: Some(format!("line {line} outside changed hunks of {file}")),
        };
    }
    CheckResult { id: Some(rid), verdict: Verdict::Ok, reason: None }
}

pub fn verify_all(manifest: &Manifest, findings: &[serde_json::Value]) -> VerdictReport {
    let results: Vec<CheckResult> = findings.iter().map(|f| check_finding(manifest, f)).collect();
    let mut summary = Summary { ok: 0, out_of_hunk: 0, unknown_file: 0, invalid_record: 0 };
    for r in &results {
        match r.verdict {
            Verdict::Ok => summary.ok += 1,
            Verdict::OutOfHunk => summary.out_of_hunk += 1,
            Verdict::UnknownFile => summary.unknown_file += 1,
            Verdict::InvalidRecord => summary.invalid_record += 1,
        }
    }
    VerdictReport { results, summary }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Bundle, Hunk, Manifest, ManifestFile, Scope, Scopes};

    fn fixture_manifest() -> Manifest {
        let empty = || Scope { in_scope: false, files: vec![] };
        Manifest {
            version: 1,
            base: "main".into(),
            head: "HEAD".into(),
            files: vec![ManifestFile {
                path: "src/a.ts".into(),
                status: 'M',
                hunks: vec![Hunk { start: 12, count: 6 }],
            }],
            scopes: Scopes {
                react_rn: empty(), tests: empty(), motion: empty(),
                navigation: empty(), sonar: empty(), rn_security: empty(),
            },
            bundles: vec![Bundle { key: "src".into(), files: vec!["src/a.ts".into()] }],
        }
    }

    fn good() -> serde_json::Value {
        serde_json::json!({
            "id": "F1", "file": "src/a.ts", "line": 14,
            "severity": "major", "title": "t", "evidence": "e",
        })
    }

    #[test]
    fn accepts_line_inside_hunk() {
        let r = check_finding(&fixture_manifest(), &good());
        assert_eq!(r, CheckResult { id: Some("F1".into()), verdict: Verdict::Ok, reason: None });
    }

    #[test]
    fn rejects_line_outside_hunks() {
        let mut f = good();
        f["line"] = serde_json::json!(5);
        assert_eq!(check_finding(&fixture_manifest(), &f).verdict, Verdict::OutOfHunk);
    }

    #[test]
    fn rejects_unknown_file() {
        let mut f = good();
        f["file"] = serde_json::json!("src/other.ts");
        assert_eq!(check_finding(&fixture_manifest(), &f).verdict, Verdict::UnknownFile);
    }

    #[test]
    fn rejects_malformed_records() {
        let mut zero = good();
        zero["line"] = serde_json::json!(0);
        assert_eq!(check_finding(&fixture_manifest(), &zero).verdict, Verdict::InvalidRecord);
        let mut sev = good();
        sev["severity"] = serde_json::json!("orange");
        assert_eq!(check_finding(&fixture_manifest(), &sev).verdict, Verdict::InvalidRecord);
        let bare = serde_json::json!({ "id": "F6" });
        assert_eq!(check_finding(&fixture_manifest(), &bare).verdict, Verdict::InvalidRecord);
    }

    #[test]
    fn summarizes_verdict_counts() {
        let mut bad_line = good();
        bad_line["line"] = serde_json::json!(5);
        let mut bad_file = good();
        bad_file["file"] = serde_json::json!("nope.ts");
        let bare = serde_json::json!({ "id": "D" });
        let out = verify_all(&fixture_manifest(), &[good(), bad_line, bad_file, bare]);
        assert_eq!(out.summary, Summary { ok: 1, out_of_hunk: 1, unknown_file: 1, invalid_record: 1 });
        assert_eq!(out.results.len(), 4);
    }
}
