#!/usr/bin/env python3
"""
update_translations.py - Extract translatable strings from QML and Rust files
and synchronize resources/translations/circuitsimulator_{en,tr}.ts.
"""

import os
import re
import shutil
import subprocess
import sys
import xml.etree.ElementTree as ET

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
QML_DIR = os.path.join(REPO_ROOT, "qml")
CRATES_DIR = os.path.join(REPO_ROOT, "crates")
TS_EN = os.path.join(REPO_ROOT, "resources", "translations", "circuitsimulator_en.ts")
TS_TR = os.path.join(REPO_ROOT, "resources", "translations", "circuitsimulator_tr.ts")

def find_lupdate():
    path = shutil.which("lupdate")
    if path:
        return path
    # Try qmake query
    qmake = shutil.which("qmake")
    if qmake:
        try:
            out = subprocess.check_output([qmake, "-query", "QT_HOST_BINS"], text=True).strip()
            candidate = os.path.join(out, "lupdate")
            if os.path.exists(candidate):
                return candidate
        except Exception:
            pass
    return None

def extract_all_strings():
    strings = set()
    # Patterns for Rust translation markers & definition patterns
    rust_patterns = [
        re.compile(r'(?:i18n::tr|cs_engine::i18n::tr|\bt)\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)'),
        re.compile(r'(?:tr_noop|cs_engine::tr_noop)\s*!\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)'),
        re.compile(r'\.with_info\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)'),
        re.compile(r'act\s*\(\s*"[^"]*"\s*,\s*"((?:[^"\\]|\\.)*)"'),
    ]
    # Patterns for QML qsTr("..."), App.tr("..."), App.translate("..."), root.tr("..."), win.tr("..."), menu.tr("..."), tr("...")
    qml_patterns = [
        re.compile(r'qsTr\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)'),
        re.compile(r'App\.tr\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)'),
        re.compile(r'App\.translate\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)'),
        re.compile(r'root\.tr\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)'),
        re.compile(r'win\.tr\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)'),
        re.compile(r'menu\.tr\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)'),
        re.compile(r'(?<!\w)tr\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)'),
    ]

    for root, _, files in os.walk(QML_DIR):
        for fname in files:
            if not fname.endswith(".qml"):
                continue
            fpath = os.path.join(root, fname)
            with open(fpath, "r", encoding="utf-8") as f:
                content = f.read()
            for pattern in qml_patterns:
                for match in pattern.finditer(content):
                    s = match.group(1).replace('\\"', '"').replace('\\\\', '\\').replace('\\n', '\n').replace('\\t', '\t').replace('\\r', '\r')
                    if s:
                        strings.add(s)

    for root, _, files in os.walk(CRATES_DIR):
        for fname in files:
            if not fname.endswith(".rs"):
                continue
            fpath = os.path.join(root, fname)
            with open(fpath, "r", encoding="utf-8") as f:
                content = f.read()
            for pattern in rust_patterns:
                for match in pattern.finditer(content):
                    s = match.group(1).replace('\\"', '"').replace('\\\\', '\\').replace('\\n', '\n').replace('\\t', '\t').replace('\\r', '\r')
                    if s:
                        strings.add(s)
    return strings

def parse_existing_ts(ts_path):
    if not os.path.exists(ts_path):
        return {}
    with open(ts_path, "r", encoding="utf-8") as f:
        content = f.read()
    
    # Simple regex extraction to preserve original layout & unfinished attrs
    messages = {}
    pattern = re.compile(r'<message[^>]*>.*?<source>(.*?)</source>.*?<translation([^>]*)>(.*?)</translation>.*?</message>', re.DOTALL)
    for m in pattern.finditer(content):
        src = m.group(1)
        attr = m.group(2)
        trans = m.group(3)
        messages[src] = (attr, trans)
    return messages

def append_missing_strings(ts_path, all_strings, is_en=False):
    if not os.path.exists(ts_path):
        return
    with open(ts_path, "r", encoding="utf-8") as f:
        content = f.read()

    # Find where existing sources are
    existing_sources = set(re.findall(r'<source>(.*?)</source>', content, re.DOTALL))
    
    new_entries = []
    for s in sorted(all_strings):
        # Escape XML
        escaped_s = s.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;').replace('"', '&quot;')
        if escaped_s not in existing_sources and s not in existing_sources:
            if is_en:
                trans = f'<translation>{escaped_s}</translation>'
            else:
                trans = '<translation type="unfinished"></translation>'
            entry = f"""    <message>
        <source>{escaped_s}</source>
        {trans}
    </message>"""
            new_entries.append(entry)

    if not new_entries:
        print(f"No missing strings for {os.path.basename(ts_path)}")
        return

    print(f"Adding {len(new_entries)} new strings to {os.path.basename(ts_path)}")
    
    insert_block = "\n" + "\n".join(new_entries)
    if "</context>" in content:
        last_ctx_end = content.rfind("</context>")
        content = content[:last_ctx_end] + insert_block + "\n" + content[last_ctx_end:]
    elif "</TS>" in content:
        last_ts_end = content.rfind("</TS>")
        block = f"""<context>
    <name>Application</name>{insert_block}
</context>
"""
        content = content[:last_ts_end] + block + content[last_ts_end:]
    
    with open(ts_path, "w", encoding="utf-8") as f:
        f.write(content)

def main():
    lupdate = find_lupdate()
    if lupdate:
        print(f"Found lupdate at: {lupdate}")
    
    all_strings = extract_all_strings()
    print(f"Found {len(all_strings)} distinct translatable strings in QML and Rust sources.")

    append_missing_strings(TS_EN, all_strings, is_en=True)
    append_missing_strings(TS_TR, all_strings, is_en=False)
    print("Translation synchronization complete.")

if __name__ == "__main__":
    main()
