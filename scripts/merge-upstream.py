#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""merge-upstream.py - merge CodexPlusPlus upstream changes to LUODA-Codex"""
import os, sys, re, shutil, subprocess
from pathlib import Path

REPO_DIR = Path(__file__).resolve().parent.parent
UPSTREAM_REMOTE = "upstream"
UPSTREAM_BRANCH = "main"
LAST_MERGE_FILE = REPO_DIR / ".last_merge_base"
BACKUP_DIR = REPO_DIR / ".merge_backups"

TEXT_EXTS = {".rs", ".tsx", ".ts", ".js", ".json", ".toml", ".yml", ".yaml",
             ".md", ".css", ".html", ".conf", ".cfg", ".ini", ".txt",
             ".sh", ".bat", ".cmd", ".ps1", ".py", ".rb", ".jsx", ".vue", ".svelte"}

PATH_MAP = {"crates/codex-plus-core": "crates/luoda-codex-core",
            "crates/codex-plus-data": "crates/luoda-codex-data",
            "apps/codex-plus-launcher": "apps/luoda-codex-launcher",
            "apps/codex-plus-manager": "apps/luoda-codex-manager"}

REPLACEMENTS = [("codex_plus_core", "luoda_codex_core"),
                ("codex_plus_data", "luoda_codex_data"),
                ("codex-plus-core", "luoda-codex-core"),
                ("codex-plus-data", "luoda-codex-data"),
                ("codex-plus-launcher", "luoda-codex-launcher"),
                ("codex-plus-manager", "luoda-codex-manager"),
                ("BigPizzaV3", "luoda2023")]

def log(msg):
    print(msg, flush=True)

def git(args):
    r = subprocess.run(["git"] + args, cwd=REPO_DIR, capture_output=True, timeout=60)
    if r.returncode != 0:
        log("Git error: " + r.stderr.decode("utf-8", errors="replace")[:300])
        sys.exit(1)
    return r.stdout

def map_path(p):
    for k, v in PATH_MAP.items():
        if p.startswith(k):
            return p.replace(k, v, 1)
    return p

def rename_text(text):
    for old, new in REPLACEMENTS:
        text = text.replace(old, new)
    return text

def rename_bytes(data):
    for old, new in REPLACEMENTS:
        data = data.replace(old.encode(), new.encode())
    return data

def is_text(path):
    return Path(path).suffix.lower() in TEXT_EXTS

def ensure_upstream():
    r = subprocess.run(["git", "remote", "get-url", UPSTREAM_REMOTE], cwd=REPO_DIR, capture_output=True)
    if r.returncode != 0:
        log("-> adding upstream remote...")
        git(["remote", "add", UPSTREAM_REMOTE, "https://github.com/BigPizzaV3/CodexPlusPlus.git"])
    log("-> fetching upstream...")
    git(["fetch", UPSTREAM_REMOTE, UPSTREAM_BRANCH])

def get_last_base():
    if LAST_MERGE_FILE.exists():
        return LAST_MERGE_FILE.read_text().strip()
    result = subprocess.run(["git", "rev-list", "--max-parents=0", "HEAD"], cwd=REPO_DIR, capture_output=True, timeout=30)
    return result.stdout.decode().strip()

def check():
    ensure_upstream()
    last = get_last_base()
    head = git(["rev-parse", UPSTREAM_REMOTE + "/" + UPSTREAM_BRANCH]).decode().strip()
    cnt = git(["rev-list", "--count", last + ".." + UPSTREAM_REMOTE + "/" + UPSTREAM_BRANCH]).decode().strip()
    log("last merge base: " + last[:12])
    log("upstream HEAD: " + head[:12])
    if cnt != "0":
        log("upstream has " + cnt + " new commits:")
        log(git(["log", "--oneline", last + ".." + UPSTREAM_REMOTE + "/" + UPSTREAM_BRANCH]).decode())
    return int(cnt)

def apply_brand_patches():
    patches = []
    app_p = REPO_DIR / "apps/luoda-codex-manager/src/App.tsx"
    if app_p.exists():
        t = app_p.read_text("utf-8")
        o = t
        t = re.sub(r'\s*\{ id: "(zedRemote|userScripts|recommendations)",.*?\},?', '', t, flags=re.DOTALL)
        t = re.sub(r'\s*\{route === "(zedRemote|userScripts|recommendations)" \?.*?\) : null\}', '', t, flags=re.DOTALL)
        t = re.sub(r' \| "(zedRemote|userScripts|recommendations)"', '', t)
        t = re.sub(r'if \(next === "(zedRemote|userScripts|recommendations)"\) \{.*?\n    \}', '', t, flags=re.DOTALL)
        t = re.sub(r'if \(next === "recommendations"\) await refreshAds\(true\);', '', t)
        t = re.sub(r'    (zedRemote|userScripts|recommendations): ".*?",\n', '', t)
        t = t.replace('brand-title">Luoda-Codex', 'brand-title">L')
        t = re.sub(r'\n{3,}', '\n\n', t)
        if t != o:
            BACKUP_DIR.mkdir(parents=True, exist_ok=True)
            shutil.copy2(app_p, BACKUP_DIR / "App.tsx.bak")
            app_p.write_text(t, "utf-8")
            patches.append("App.tsx")

    main_p = REPO_DIR / "apps/luoda-codex-manager/src/main.tsx"
    if main_p.exists():
        t = main_p.read_text("utf-8"); o = t
        t = t.replace('Luoda-Codex', 'L')
        if t != o: main_p.write_text(t, "utf-8"); patches.append("main.tsx")

    css_p = REPO_DIR / "apps/luoda-codex-manager/src/styles.css"
    if css_p.exists():
        t = css_p.read_text("utf-8"); o = t
        t = t.replace('Luoda-Codex', 'L')
        if t != o: css_p.write_text(t, "utf-8"); patches.append("styles.css")

    if patches:
        log("  brand patches: " + ", ".join(patches))

def apply():
    ensure_upstream()
    last = get_last_base()
    head = git(["rev-parse", UPSTREAM_REMOTE + "/" + UPSTREAM_BRANCH]).decode().strip()
    log("last merge: " + last[:12])
    log("upstream HEAD: " + head[:12])

    if last == head:
        log("no new changes"); apply_brand_patches(); return

    raw = git(["diff", "--name-status", last + ".." + UPSTREAM_REMOTE + "/" + UPSTREAM_BRANCH])
    files = []
    for line in raw.decode().strip().split("\n"):
        line = line.strip()
        if not line: continue
        parts = line.split("\t", 1)
        if len(parts) == 2: files.append((parts[0], parts[1]))

    log(str(len(files)) + " files changed")
    up_commit = UPSTREAM_REMOTE + "/" + UPSTREAM_BRANCH
    changed = 0

    for status, up_path in files:
        local_path = map_path(up_path)
        if not local_path: continue
        full_local = REPO_DIR / local_path

        if status.startswith("D"):
            if full_local.exists(): full_local.unlink(); log("  D " + local_path)
            continue

        result = subprocess.run(["git", "show", up_commit + ":" + up_path], cwd=REPO_DIR, capture_output=True, timeout=30)
        if result.returncode != 0: log("  ? " + up_path); continue
        cb = result.stdout

        if is_text(local_path):
            try: text = cb.decode("utf-8")
            except UnicodeDecodeError: text = cb.decode("utf-8", errors="replace")
            nt = rename_text(text)
            cb = (nt if nt != text else text).encode("utf-8")
        else:
            cb = rename_bytes(cb)

        if full_local.exists():
            BACKUP_DIR.mkdir(parents=True, exist_ok=True)
            shutil.copy2(full_local, BACKUP_DIR / (local_path.replace("/", "_") + ".bak"))

        full_local.parent.mkdir(parents=True, exist_ok=True)
        full_local.write_bytes(cb)
        log("  " + ("+ " if status.startswith("A") else "M ") + local_path)
        changed += 1

    apply_brand_patches()
    LAST_MERGE_FILE.write_text(head)
    log("merge base updated to " + head[:12])

    if changed > 0:
        log("\n=== Verify ===")
        r = subprocess.run(["cargo", "check", "-p", "luoda-codex-core"], cwd=REPO_DIR, capture_output=True, timeout=120)
        if r.returncode != 0:
            log("!! check failed, backups: " + str(BACKUP_DIR))
            sys.exit(1)
        log("  cargo check: OK")
        r = subprocess.run(["cargo", "test", "-p", "luoda-codex-core", "--test", "bridge_routes"], cwd=REPO_DIR, capture_output=True, timeout=120)
        if r.returncode != 0:
            log("!! test failed, backups: " + str(BACKUP_DIR))
            sys.exit(1)
        log("  cargo test: OK")
        log("Merge verified OK")

if __name__ == "__main__":
    if "--apply" in sys.argv: apply()
    elif "--check" in sys.argv: check()
    elif "--verify-only" in sys.argv:
        r = subprocess.run(["cargo", "check", "-p", "luoda-codex-core"], cwd=REPO_DIR, capture_output=True, timeout=120)
        sys.exit(r.returncode)
    else:
        n = check()
        if n > 0: log("Run --apply to merge")
