use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hunk {
    pub start: u32,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFile {
    pub path: String,
    pub hunks: Vec<Hunk>,
    pub added: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    pub status: char,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub in_scope: bool,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scopes {
    pub react_rn: Scope,
    pub tests: Scope,
    pub motion: Scope,
    pub navigation: Scope,
    pub sonar: Scope,
    pub rn_security: Scope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    pub key: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub base: String,
    pub head: String,
    pub files: Vec<ManifestFile>,
    pub scopes: Scopes,
    pub bundles: Vec<Bundle>,
}

// Parse NUL-separated `git diff --name-status -z` output.
// Returns map of new-path -> status letter. Renames resolve to the new path.
pub fn parse_name_status(text: &str) -> BTreeMap<String, char> {
    let parts: Vec<&str> = text.split('\0').collect();
    let mut status = BTreeMap::new();
    let mut i = 0;
    while i < parts.len() {
        let code = parts[i];
        i += 1;
        if code.is_empty() {
            continue;
        }
        let kind = code.trim().chars().next().unwrap_or('?');
        if kind == 'R' || kind == 'C' {
            i += 1; // skip old path
            if let Some(to) = parts.get(i) {
                i += 1;
                if !to.is_empty() {
                    status.insert(to.to_string(), kind);
                }
            }
        } else if let Some(p) = parts.get(i) {
            i += 1;
            if !p.is_empty() {
                status.insert(p.to_string(), kind);
            }
        }
    }
    status
}

// "@@ -a[,b] +c[,d] @@" -> Hunk { start: c, count: d or 1 }. None when not a hunk header.
fn parse_hunk_header(raw: &str) -> Option<Hunk> {
    let rest = raw.strip_prefix("@@ ")?;
    let end = rest.find(" @@")?;
    let ranges = &rest[..end];
    let mut parts = ranges.split_whitespace();
    parts.next()?; // old range
    let new_range = parts.next()?;
    let nums = new_range.strip_prefix('+')?;
    let (start_s, count_s) = match nums.split_once(',') {
        Some((s, c)) => (s, Some(c)),
        None => (nums, None),
    };
    let start = start_s.parse::<u32>().ok()?;
    let count = match count_s {
        Some(c) => c.parse::<u32>().ok()?,
        None => 1,
    };
    Some(Hunk { start, count })
}

// Parse unified diff text (`git diff -U0`).
// Deleted files are dropped. Binary files keep zero hunks.
// Paths with spaces/quotes are not unquoted (see README Known limits).
pub fn parse_diff(diff_text: &str) -> Vec<ParsedFile> {
    let mut files: Vec<ParsedFile> = Vec::new();
    let mut current: Option<ParsedFile> = None;
    for raw in diff_text.split('\n') {
        if let Some(rest) = raw.strip_prefix("diff --git ") {
            if let Some(f) = current.take() {
                files.push(f);
            }
            let path = rest.rsplit(" b/").next().unwrap_or("");
            if path.is_empty() || path == "/dev/null" {
                current = None;
            } else {
                current = Some(ParsedFile { path: path.to_string(), hunks: Vec::new(), added: Vec::new() });
            }
            continue;
        }
        let Some(f) = current.as_mut() else { continue };
        if let Some(plus) = raw.strip_prefix("+++ ") {
            let p = plus.trim().strip_prefix("b/").unwrap_or(plus.trim());
            if p == "/dev/null" {
                current = None;
            }
            continue;
        }
        if let Some(h) = parse_hunk_header(raw) {
            f.hunks.push(h);
            continue;
        }
        if let Some(added) = raw.strip_prefix('+') {
            f.added.push(added.to_string());
        }
    }
    if let Some(f) = current.take() {
        files.push(f);
    }
    files
}

pub const MOTION_TRIGGERS: &[&str] = &[
    "useAnimatedStyle", "useAnimatedReaction", "useSharedValue",
    "withSpring", "withTiming", "withDecay", "useDerivedValue",
    "useAnimatedScrollHandler", "scheduleOnUI", "scheduleOnRN",
    "@gorhom/bottom-sheet", "footerComponent",
    "@keyframes", "animation:", "transition",
    "framer-motion", "gsap",
];

fn is_test(p: &str) -> bool {
    p.contains(".test.") || p.contains("__tests__") || p.contains("__snapshots__")
}
fn is_locales(p: &str) -> bool {
    p.contains("/locales/")
}
fn is_ts(p: &str) -> bool {
    p.ends_with(".ts") || p.ends_with(".tsx")
}
fn is_nav(p: &str) -> bool {
    p.ends_with("Screen.tsx") || p.ends_with("-screen.tsx") || p.contains("navigator") || p.ends_with("navigation-type.ts")
}
fn is_sonar(p: &str) -> bool {
    [".ts", ".tsx", ".js", ".jsx", ".py", ".java"].iter().any(|e| p.ends_with(e))
}
fn file_name(p: &str) -> &str {
    p.rsplit('/').next().unwrap_or(p)
}
fn is_sec_config(p: &str) -> bool {
    let n = file_name(p);
    matches!(n, "package.json" | "package-lock.json" | "app.json" | "Podfile" | "AndroidManifest.xml")
        || n.starts_with("app.config.")
        || n.ends_with(".plist")
        || n.ends_with(".gradle")
}
fn is_sec_dir(p: &str) -> bool {
    p.starts_with("android/") || p.starts_with("ios/")
}

fn wrap(list: Vec<String>) -> Scope {
    Scope { in_scope: !list.is_empty(), files: list }
}

// Classify scope flags mirroring the skill pass rules.
// Over-triggering is the safe direction: a wrongly spawned pass costs
// tokens, a missed pass loses coverage.
pub fn classify_scopes(files: &[ParsedFile]) -> Scopes {
    let collect = |f: &dyn Fn(&ParsedFile) -> bool| -> Vec<String> {
        files.iter().filter(|x| f(x)).map(|x| x.path.clone()).collect()
    };
    Scopes {
        react_rn: wrap(collect(&|f| is_ts(&f.path) && !is_test(&f.path) && !is_locales(&f.path))),
        tests: wrap(collect(&|f| is_test(&f.path))),
        motion: wrap(collect(&|f| {
            MOTION_TRIGGERS.iter().any(|t| f.added.iter().any(|l| l.contains(t)))
        })),
        navigation: wrap(collect(&|f| is_nav(&f.path))),
        sonar: wrap(collect(&|f| is_sonar(&f.path) && !is_test(&f.path) && !is_locales(&f.path))),
        rn_security: wrap(collect(&|f| {
            (!is_test(&f.path) && !is_locales(&f.path) && is_ts(&f.path))
                || is_sec_config(&f.path)
                || is_sec_dir(&f.path)
        })),
    }
}

// Group changed files for per-pass scoping. BTreeMap keeps keys sorted.
pub fn bundle_files(paths: &[String]) -> Vec<Bundle> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for p in paths {
        let segs: Vec<&str> = p.split('/').collect();
        let key = if segs.len() == 1 {
            "(root)".to_string()
        } else if segs.len() == 2 {
            segs[0].to_string()
        } else {
            format!("{}/{}", segs[0], segs[1])
        };
        groups.entry(key).or_default().push(p.clone());
    }
    groups.into_iter().map(|(key, mut list)| {
        list.sort();
        Bundle { key, files: list }
    }).collect()
}

// Build the full manifest by shelling out to git. Only impure part;
// covered by CLI smoke below, not by unit tests.
pub fn build_manifest(repo: &str, base: &str, head: &str) -> io::Result<Manifest> {
    let range = format!("{base}...{head}");
    let status_out = Command::new("git")
        .args(["diff", &range, "--name-status", "-z"])
        .current_dir(repo).output()?;
    if !status_out.status.success() {
        return Err(io::Error::other(format!("git diff --name-status failed for {range}")));
    }
    let status = parse_name_status(&String::from_utf8_lossy(&status_out.stdout));
    let diff_out = Command::new("git")
        .args(["diff", &range, "-U0", "--no-color"])
        .current_dir(repo).output()?;
    if !diff_out.status.success() {
        return Err(io::Error::other(format!("git diff failed for {range}")));
    }
    let parsed = parse_diff(&String::from_utf8_lossy(&diff_out.stdout));
    let files: Vec<ManifestFile> = parsed.iter().map(|f| ManifestFile {
        path: f.path.clone(),
        status: status.get(&f.path).copied().unwrap_or('M'),
        hunks: f.hunks.clone(),
    }).collect();
    let scopes = classify_scopes(&parsed);
    let paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
    let bundles = bundle_files(&paths);
    Ok(Manifest { version: 1, base: base.to_string(), head: head.to_string(), files, scopes, bundles })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = r#"diff --git a/src/PlayerScreen.tsx b/src/PlayerScreen.tsx
index 1111111..2222222 100644
--- a/src/PlayerScreen.tsx
+++ b/src/PlayerScreen.tsx
@@ -10,3 +12,6 @@ export function Player() {
   const x = 1;
+import { useAnimatedStyle } from 'react-native-reanimated';
+const style = useAnimatedStyle(() => ({}));
   return null;
diff --git a/src/format.test.ts b/src/format.test.ts
index 3333333..4444444 100644
--- a/src/format.test.ts
+++ b/src/format.test.ts
@@ -1,2 +1,3 @@
 import { format } from './format';
+it('pads', () => {});
"#;

    #[test]
    fn parses_files_hunks_and_added_lines() {
        let files = parse_diff(SAMPLE_DIFF);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "src/PlayerScreen.tsx");
        assert_eq!(files[0].hunks, vec![Hunk { start: 12, count: 6 }]);
        assert!(files[0].added.iter().any(|l| l.contains("useAnimatedStyle")));
        assert_eq!(files[1].path, "src/format.test.ts");
        assert_eq!(files[1].hunks, vec![Hunk { start: 1, count: 3 }]);
    }

    #[test]
    fn drops_deleted_files() {
        let diff = "diff --git a/src/old.ts b/src/old.ts\ndeleted file mode 100644\nindex aaaaaaa..0000000\n--- a/src/old.ts\n+++ /dev/null\n@@ -1,2 +0,0 @@\n-const a = 1;\n";
        assert!(parse_diff(diff).is_empty());
    }

    #[test]
    fn parses_name_status_with_renames() {
        let status = parse_name_status("M\0src/a.ts\0R100\0src/old.ts\0src/new.ts\0");
        assert_eq!(status.get("src/a.ts"), Some(&'M'));
        assert_eq!(status.get("src/new.ts"), Some(&'R'));
        assert!(!status.contains_key("src/old.ts"));
    }

    fn scope_fixture() -> Vec<ParsedFile> {
        vec![
            ParsedFile { path: "src/PlayerScreen.tsx".into(), hunks: vec![], added: vec!["import { useAnimatedStyle } from \"x\"".into()] },
            ParsedFile { path: "src/format.test.ts".into(), hunks: vec![], added: vec!["it(\"pads\", () => {})".into()] },
            ParsedFile { path: "android/app/build.gradle".into(), hunks: vec![], added: vec!["compileSdk 35".into()] },
        ]
    }

    fn scope(files: &[&str]) -> Scope {
        Scope { in_scope: !files.is_empty(), files: files.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn classifies_react_tests_motion_navigation() {
        let s = classify_scopes(&scope_fixture());
        assert_eq!(s.react_rn, scope(&["src/PlayerScreen.tsx"]));
        assert_eq!(s.tests, scope(&["src/format.test.ts"]));
        assert_eq!(s.motion, scope(&["src/PlayerScreen.tsx"]));
        assert_eq!(s.navigation, scope(&["src/PlayerScreen.tsx"]));
    }

    #[test]
    fn classifies_sonar_and_security() {
        let s = classify_scopes(&scope_fixture());
        assert_eq!(s.sonar, scope(&["src/PlayerScreen.tsx"]));
        assert_eq!(s.rn_security, scope(&["src/PlayerScreen.tsx", "android/app/build.gradle"]));
    }

    #[test]
    fn bundles_group_by_directory() {
        let paths = ["src/a.ts", "src/b.ts", "docs/x/y.md", "README.md"]
            .iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // BTreeMap keeps bundle keys sorted.
        assert_eq!(bundle_files(&paths), vec![
            Bundle { key: "(root)".into(), files: vec!["README.md".into()] },
            Bundle { key: "docs/x".into(), files: vec!["docs/x/y.md".into()] },
            Bundle { key: "src".into(), files: vec!["src/a.ts".into(), "src/b.ts".into()] },
        ]);
    }
}
