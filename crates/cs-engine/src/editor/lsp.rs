//! clangd JSON-RPC client. C++ `LspClient`: framing, initialize, didOpen /
//! didChange, completion, diagnostics, hover, definition.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use super::compiler::command_exists;
use crate::theme::{ColorId, ColorTheme};

const INO_PREAMBLE: &str = "#include <Arduino.h>\n";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub detail: String,
    pub insert_text: String,
    pub kind: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticItem {
    pub start_line: i32,
    pub start_col: i32,
    pub end_line: i32,
    pub end_col: i32,
    pub severity: i32,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SignatureHelp {
    pub label: String,
    pub active_parameter: i32,
    pub parameters: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum LspEvent {
    Completions(Vec<CompletionItem>),
    Diagnostics {
        uri: String,
        items: Vec<DiagnosticItem>,
    },
    Hover(String),
    Definition {
        uri: String,
        line: i32,
        character: i32,
    },
    SignatureHelp(Option<SignatureHelp>),
    Log(String),
}

struct Shared {
    initialized: AtomicBool,
    last_completion: AtomicI32,
    last_hover: AtomicI32,
    last_definition: AtomicI32,
    last_signature: AtomicI32,
    events: Mutex<Vec<LspEvent>>,
    pending_open: Mutex<Vec<PendingDoc>>,
    versions: Mutex<HashMap<String, i32>>,
    offsets: Mutex<HashMap<String, i32>>,
}

struct PendingDoc {
    uri: String,
    text: String,
    language_id: String,
}

pub struct LspClient {
    child: Option<Child>,
    stdin: Option<Arc<Mutex<ChildStdin>>>,
    shared: Arc<Shared>,
    next_id: AtomicI32,
}

impl Default for LspClient {
    fn default() -> Self {
        Self {
            child: None,
            stdin: None,
            shared: Arc::new(Shared {
                initialized: AtomicBool::new(false),
                last_completion: AtomicI32::new(-1),
                last_hover: AtomicI32::new(-1),
                last_definition: AtomicI32::new(-1),
                last_signature: AtomicI32::new(-1),
                events: Mutex::new(Vec::new()),
                pending_open: Mutex::new(Vec::new()),
                versions: Mutex::new(HashMap::new()),
                offsets: Mutex::new(HashMap::new()),
            }),
            next_id: AtomicI32::new(1),
        }
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn find_clangd() -> Option<PathBuf> {
    for name in [
        "clangd",
        "clangd-20",
        "clangd-19",
        "clangd-18",
        "clangd-17",
        "clangd-16",
    ] {
        if command_exists(name) {
            return Some(PathBuf::from(name));
        }
    }
    for p in [
        "/opt/homebrew/bin/clangd",
        "/usr/local/bin/clangd",
        "/usr/bin/clangd",
    ] {
        if Path::new(p).is_file() {
            return Some(PathBuf::from(p));
        }
    }
    None
}

pub fn language_id_for_path(path: &str) -> Option<&'static str> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".ino")
        || lower.ends_with(".cpp")
        || lower.ends_with(".cxx")
        || lower.ends_with(".cc")
        || lower.ends_with(".h")
        || lower.ends_with(".hpp")
        || lower.ends_with(".hxx")
        || lower.ends_with(".hh")
    {
        Some("cpp")
    } else if lower.ends_with(".c") {
        Some("c")
    } else {
        None
    }
}

pub fn to_uri(path: &str) -> String {
    if path.starts_with("file://") {
        return path.to_string();
    }
    if path.is_empty() {
        return String::new();
    }
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
    let s = abs.to_string_lossy();
    if cfg!(windows) {
        format!("file:///{}", s.replace('\\', "/"))
    } else {
        format!("file://{s}")
    }
}

pub fn uri_to_path(uri: &str) -> String {
    let s = uri.strip_prefix("file://").unwrap_or(uri);
    #[cfg(windows)]
    {
        let s = s.trim_start_matches('/');
        return s.replace('/', "\\");
    }
    #[cfg(not(windows))]
    {
        s.to_string()
    }
}

fn ino_offset(uri: &str) -> i32 {
    if uri.to_ascii_lowercase().ends_with(".ino") {
        1
    } else {
        0
    }
}

pub fn encode_message(json: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(json).unwrap_or_default();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut out = header.into_bytes();
    out.extend_from_slice(&body);
    out
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn content_length(header: &[u8]) -> Option<usize> {
    let s = String::from_utf8_lossy(header);
    for line in s.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

/// Drain complete LSP messages from `buf`.
pub fn decode_messages(buf: &mut Vec<u8>) -> Vec<Value> {
    let mut out = Vec::new();
    loop {
        let Some(header_end) = find_header_end(buf) else {
            break;
        };
        let Some(len) = content_length(&buf[..header_end]) else {
            buf.drain(..header_end + 4);
            continue;
        };
        let start = header_end + 4;
        if buf.len() < start + len {
            break;
        }
        let payload = buf[start..start + len].to_vec();
        buf.drain(..start + len);
        if let Ok(v) = serde_json::from_slice(&payload) {
            out.push(v);
        }
    }
    out
}

fn parse_hover(contents: &Value) -> String {
    if let Some(s) = contents.as_str() {
        return s.to_string();
    }
    if let Some(obj) = contents.as_object() {
        if let Some(v) = obj.get("value").and_then(|v| v.as_str()) {
            if let Some(lang) = obj.get("language").and_then(|v| v.as_str()) {
                return format!("```{lang}\n{v}\n```");
            }
            return v.to_string();
        }
    }
    if let Some(arr) = contents.as_array() {
        return arr
            .iter()
            .map(parse_hover)
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
    }
    String::new()
}

fn push_event(shared: &Shared, ev: LspEvent) {
    if let Ok(mut g) = shared.events.lock() {
        g.push(ev);
    }
}

fn send_stdin(stdin: &Arc<Mutex<ChildStdin>>, msg: &Value) {
    let bytes = encode_message(msg);
    if let Ok(mut s) = stdin.lock() {
        let _ = s.write_all(&bytes);
        let _ = s.flush();
    }
}

fn handle_message(msg: &Value, shared: &Shared, stdin: &Arc<Mutex<ChildStdin>>) {
    let method = msg.get("method").and_then(|v| v.as_str()).unwrap_or("");
    if method == "textDocument/publishDiagnostics" {
        let Some(params) = msg.get("params") else {
            return;
        };
        let uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let offset = shared
            .offsets
            .lock()
            .ok()
            .and_then(|g| g.get(&uri).copied())
            .unwrap_or(0);
        let mut items = Vec::new();
        if let Some(arr) = params.get("diagnostics").and_then(|v| v.as_array()) {
            for d in arr {
                let start = d.get("range").and_then(|r| r.get("start"));
                let end = d.get("range").and_then(|r| r.get("end"));
                let start_line = start
                    .and_then(|s| s.get("line"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                if start_line < offset {
                    continue;
                }
                items.push(DiagnosticItem {
                    start_line: start_line - offset,
                    start_col: start
                        .and_then(|s| s.get("character"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32,
                    end_line: end
                        .and_then(|s| s.get("line"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32
                        - offset,
                    end_col: end
                        .and_then(|s| s.get("character"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32,
                    severity: d.get("severity").and_then(|v| v.as_i64()).unwrap_or(1) as i32,
                    message: d
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
        push_event(shared, LspEvent::Diagnostics { uri, items });
        return;
    }

    let Some(id) = msg.get("id").and_then(|v| v.as_i64()).map(|n| n as i32) else {
        return;
    };
    if id == 1 && !shared.initialized.load(Ordering::SeqCst) {
        shared.initialized.store(true, Ordering::SeqCst);
        send_stdin(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "initialized",
                "params": {}
            }),
        );
        let pending = shared
            .pending_open
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default();
        for doc in pending {
            if let Ok(mut v) = shared.versions.lock() {
                v.insert(doc.uri.clone(), 1);
            }
            send_stdin(
                stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": doc.uri,
                            "languageId": doc.language_id,
                            "version": 1,
                            "text": doc.text
                        }
                    }
                }),
            );
        }
        return;
    }

    let result = msg.get("result");
    if id == shared.last_completion.load(Ordering::SeqCst) {
        let mut items = Vec::new();
        let arr = result.and_then(|r| {
            if let Some(a) = r.as_array() {
                Some(a.clone())
            } else {
                r.get("items").and_then(|v| v.as_array()).cloned()
            }
        });
        if let Some(arr) = arr {
            for it in arr {
                let label = it
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let detail = it
                    .get("detail")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let kind = it.get("kind").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let insert = if let Some(t) = it
                    .get("textEdit")
                    .and_then(|e| e.get("newText"))
                    .and_then(|v| v.as_str())
                {
                    t.trim().to_string()
                } else if let Some(t) = it.get("insertText").and_then(|v| v.as_str()) {
                    t.trim().to_string()
                } else {
                    label.clone()
                };
                if !label.is_empty() {
                    items.push(CompletionItem {
                        label,
                        detail,
                        insert_text: insert,
                        kind,
                    });
                }
            }
        }
        push_event(shared, LspEvent::Completions(items));
    } else if id == shared.last_hover.load(Ordering::SeqCst) {
        let text = result
            .and_then(|r| r.get("contents"))
            .map(parse_hover)
            .unwrap_or_default();
        push_event(shared, LspEvent::Hover(text));
    } else if id == shared.last_definition.load(Ordering::SeqCst) {
        let loc = result.and_then(|r| {
            if let Some(arr) = r.as_array() {
                arr.first().cloned()
            } else {
                Some(r.clone())
            }
        });
        if let Some(loc) = loc {
            let uri = loc
                .get("uri")
                .or_else(|| loc.get("targetUri"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let range = loc
                .get("range")
                .or_else(|| loc.get("targetRange"))
                .or_else(|| loc.get("targetSelectionRange"));
            let start = range.and_then(|r| r.get("start"));
            let line = start
                .and_then(|s| s.get("line"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let character = start
                .and_then(|s| s.get("character"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let offset = shared
                .offsets
                .lock()
                .ok()
                .and_then(|g| g.get(&uri).copied())
                .unwrap_or(0);
            if !uri.is_empty() {
                push_event(
                    shared,
                    LspEvent::Definition {
                        uri,
                        line: (line - offset).max(0),
                        character,
                    },
                );
            }
        }
    } else if id == shared.last_signature.load(Ordering::SeqCst) {
        let sig = result.and_then(|r| {
            let sigs = r.get("signatures")?.as_array()?;
            if sigs.is_empty() {
                return None;
            }
            let active_sig = r
                .get("activeSignature")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as usize;
            let sig_obj = sigs.get(active_sig.min(sigs.len() - 1))?;
            let label = sig_obj.get("label")?.as_str()?.to_string();
            let active_param = sig_obj
                .get("activeParameter")
                .and_then(|v| v.as_i64())
                .or_else(|| r.get("activeParameter").and_then(|v| v.as_i64()))
                .unwrap_or(0) as i32;
            let mut parameters = Vec::new();
            if let Some(params) = sig_obj.get("parameters").and_then(|v| v.as_array()) {
                for p in params {
                    let p_label = p.get("label");
                    if let Some(s) = p_label.and_then(|v| v.as_str()) {
                        parameters.push(s.to_string());
                    } else if let Some(arr) = p_label.and_then(|v| v.as_array()) {
                        if arr.len() == 2 {
                            let start = arr[0].as_u64().unwrap_or(0) as usize;
                            let end = arr[1].as_u64().unwrap_or(0) as usize;
                            if end <= label.len() && start <= end {
                                parameters.push(label[start..end].to_string());
                            }
                        }
                    }
                }
            }
            Some(SignatureHelp {
                label,
                active_parameter: active_param,
                parameters,
            })
        });
        push_event(shared, LspEvent::SignatureHelp(sig));
    }
}

impl LspClient {
    pub fn is_running(&self) -> bool {
        self.stdin.is_some()
    }

    pub fn is_initialized(&self) -> bool {
        self.shared.initialized.load(Ordering::SeqCst)
    }

    pub fn start(&mut self, server: &Path, root: &str) {
        if self.stdin.is_some() {
            return;
        }
        let mut child = match Command::new(server)
            .args(["--header-insertion=never", "--query-driver=**"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                push_event(
                    &self.shared,
                    LspEvent::Log(format!("[LSP] Failed to start {}: {e}", server.display())),
                );
                return;
            }
        };
        let stdin = match child.stdin.take() {
            Some(s) => Arc::new(Mutex::new(s)),
            None => return,
        };
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        self.stdin = Some(stdin.clone());
        self.child = Some(child);
        self.shared.initialized.store(false, Ordering::SeqCst);
        self.next_id.store(2, Ordering::SeqCst);

        if let Some(mut stdout) = stdout {
            let shared = self.shared.clone();
            let stdin_r = stdin.clone();
            thread::Builder::new()
                .name("cs-lsp-stdout".into())
                .spawn(move || {
                    let mut buf = Vec::new();
                    let mut tmp = [0u8; 8192];
                    loop {
                        match stdout.read(&mut tmp) {
                            Ok(0) => break,
                            Ok(n) => {
                                buf.extend_from_slice(&tmp[..n]);
                                for msg in decode_messages(&mut buf) {
                                    handle_message(&msg, shared.as_ref(), &stdin_r);
                                }
                            }
                            Err(_) => break,
                        }
                    }
                })
                .ok();
        }
        if let Some(mut stderr) = stderr {
            let shared = self.shared.clone();
            thread::Builder::new()
                .name("cs-lsp-stderr".into())
                .spawn(move || {
                    let mut acc = String::new();
                    let mut b = [0u8; 2048];
                    loop {
                        match stderr.read(&mut b) {
                            Ok(0) => break,
                            Ok(n) => {
                                acc.push_str(&String::from_utf8_lossy(&b[..n]));
                                while let Some(i) = acc.find('\n') {
                                    let line = acc[..i].trim().to_string();
                                    acc.drain(..=i);
                                    if !line.is_empty() {
                                        push_event(
                                            &shared,
                                            LspEvent::Log(format!("[clangd stderr] {line}")),
                                        );
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                })
                .ok();
        }

        let root_uri = to_uri(root);
        let pid = std::process::id() as i32;
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": pid,
                "rootUri": root_uri,
                "clientInfo": { "name": "Circuit Simulator" },
                "initializationOptions": {
                    "fallbackFlags": ["-x", "c++", "-std=gnu++17"]
                },
                "capabilities": {
                    "textDocument": {
                        "completion": { "completionItem": { "snippetSupport": false } },
                        "hover": {},
                        "definition": {},
                        "signatureHelp": {
                            "signatureInformation": {
                                "parameterInformation": { "labelOffsetSupport": true }
                            }
                        },
                        "publishDiagnostics": {}
                    }
                }
            }
        }));
    }

    pub fn stop(&mut self) {
        if let Some(stdin) = &self.stdin {
            send_stdin(
                stdin,
                &json!({ "jsonrpc": "2.0", "id": 999999, "method": "shutdown" }),
            );
            send_stdin(stdin, &json!({ "jsonrpc": "2.0", "method": "exit" }));
        }
        self.stdin = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.shared.initialized.store(false, Ordering::SeqCst);
    }

    fn send(&self, msg: &Value) {
        if let Some(stdin) = &self.stdin {
            send_stdin(stdin, msg);
        }
    }

    pub fn take_events(&self) -> Vec<LspEvent> {
        self.shared
            .events
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default()
    }

    pub fn open_file(&self, path: &str, text: &str) {
        let Some(lang) = language_id_for_path(path) else {
            return;
        };
        if self.stdin.is_none() {
            return;
        }
        let uri = to_uri(path);
        if uri.is_empty() {
            return;
        }
        let offset = ino_offset(&uri);
        if let Ok(mut o) = self.shared.offsets.lock() {
            o.insert(uri.clone(), offset);
        }
        let sent = if offset > 0 {
            format!("{INO_PREAMBLE}{text}")
        } else {
            text.to_string()
        };
        if !self.shared.initialized.load(Ordering::SeqCst) {
            if let Ok(mut p) = self.shared.pending_open.lock() {
                p.retain(|d| d.uri != uri);
                p.push(PendingDoc {
                    uri,
                    text: sent,
                    language_id: lang.into(),
                });
            }
            return;
        }
        if let Ok(mut v) = self.shared.versions.lock() {
            v.insert(uri.clone(), 1);
        }
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": lang,
                    "version": 1,
                    "text": sent
                }
            }
        }));
    }

    pub fn close_file(&self, path: &str) {
        let uri = to_uri(path);
        if uri.is_empty() {
            return;
        }
        if self.shared.initialized.load(Ordering::SeqCst) {
            self.send(&json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": {
                    "textDocument": { "uri": uri }
                }
            }));
        }
    }

    pub fn reopen_file(&self, path: &str, text: &str) {
        self.close_file(path);
        self.open_file(path, text);
    }

    pub fn change_file(&self, path: &str, text: &str) {
        if !self.shared.initialized.load(Ordering::SeqCst) {
            self.open_file(path, text);
            return;
        }
        let uri = to_uri(path);
        if uri.is_empty() {
            return;
        }
        let offset = self
            .shared
            .offsets
            .lock()
            .ok()
            .and_then(|g| g.get(&uri).copied())
            .unwrap_or_else(|| ino_offset(&uri));
        let sent = if offset > 0 {
            format!("{INO_PREAMBLE}{text}")
        } else {
            text.to_string()
        };
        let version = {
            let mut g = self.shared.versions.lock().unwrap();
            let e = g.entry(uri.clone()).or_insert(1);
            *e += 1;
            *e
        };
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": version },
                "contentChanges": [{ "text": sent }]
            }
        }));
    }

    fn position_msg(
        &self,
        method: &str,
        path: &str,
        line: i32,
        character: i32,
        last: &AtomicI32,
    ) -> i32 {
        let uri = to_uri(path);
        if uri.is_empty() || !self.shared.initialized.load(Ordering::SeqCst) {
            return -1;
        }
        let offset = self
            .shared
            .offsets
            .lock()
            .ok()
            .and_then(|g| g.get(&uri).copied())
            .unwrap_or(0);
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        last.store(id, Ordering::SeqCst);
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line + offset, "character": character }
            }
        }));
        id
    }

    pub fn request_completion(&self, path: &str, line: i32, character: i32) {
        self.position_msg(
            "textDocument/completion",
            path,
            line,
            character,
            &self.shared.last_completion,
        );
    }

    pub fn request_hover(&self, path: &str, line: i32, character: i32) {
        self.position_msg(
            "textDocument/hover",
            path,
            line,
            character,
            &self.shared.last_hover,
        );
    }

    pub fn request_definition(&self, path: &str, line: i32, character: i32) {
        self.position_msg(
            "textDocument/definition",
            path,
            line,
            character,
            &self.shared.last_definition,
        );
    }

    pub fn request_signature_help(&self, path: &str, line: i32, character: i32) {
        self.position_msg(
            "textDocument/signatureHelp",
            path,
            line,
            character,
            &self.shared.last_signature,
        );
    }
}

// Format a SignatureHelp into HTML with the active parameter bolded.
// Port of C++ `CodeEditor::formatSignatureHtml`.
pub fn format_signature_html(sig: &SignatureHelp) -> String {
    if sig.label.is_empty() {
        return String::new();
    }
    let open = sig.label.find('(');
    let close = sig.label.rfind(')');
    let (prefix, param_str, suffix) = match (open, close) {
        (Some(o), Some(c)) if c > o => (&sig.label[..=o], &sig.label[o + 1..c], &sig.label[c..]),
        _ => {
            return format!("<p style='white-space:pre'>{}</p>", html_escape(&sig.label));
        }
    };

    // Split parameters at top-level commas, respecting <>()[] nesting.
    let mut params = Vec::new();
    let mut nesting = 0i32;
    let mut current = String::new();
    for ch in param_str.chars() {
        match ch {
            '<' | '(' | '[' => nesting += 1,
            '>' | ')' | ']' => {
                if nesting > 0 {
                    nesting -= 1;
                }
            }
            ',' if nesting == 0 => {
                params.push(current.clone());
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(ch);
    }
    params.push(current);

    let formatted: Vec<String> = params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i as i32 == sig.active_parameter {
                let leading_len = p.len() - p.trim_start().len();
                let trailing_len = p.len() - p.trim_end().len();
                let leading = &p[..leading_len];
                let middle = &p[leading_len..p.len() - trailing_len];
                let trailing = &p[p.len() - trailing_len..];
                format!("{leading}<b>{}</b>{trailing}", html_escape(middle))
            } else {
                html_escape(p)
            }
        })
        .collect();

    format!(
        "<p style='white-space:pre-wrap; max-width:600px;'>{}{}{}</p>",
        html_escape(prefix),
        formatted.join(","),
        html_escape(suffix),
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Format LSP hover markdown into styled HTML for display in tooltips.
/// Parity with C++ `CodeEditor::formatHoverMarkdown`.
pub fn format_hover_markdown(md: &str, dark: bool) -> String {
    if md.trim().is_empty() {
        return String::new();
    }

    let code_bg = ColorTheme::get_hex(ColorId::EditorCodeBg, dark);
    let code_border = ColorTheme::get_hex(ColorId::EditorCodeBorder, dark);
    let code_color = ColorTheme::get_hex(ColorId::EditorCodeText, dark);
    let text_color = ColorTheme::get_hex(ColorId::EditorText, dark);

    let mut html = String::new();
    let mut in_code_block = false;
    let mut code_block_text = String::new();

    for line in md.lines() {
        let trimmed_start = line.trim_start();
        if trimmed_start.starts_with("```") {
            if in_code_block {
                let escaped_code = html_escape(code_block_text.trim());
                html.push_str(&format!(
                    "<pre style='margin: 4px 0; padding: 4px 6px; background-color: {code_bg}; border: 1px solid {code_border}; border-radius: 4px; font-family: monospace; color: {code_color};'><code>{escaped_code}</code></pre>"
                ));
                code_block_text.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
                code_block_text.clear();
            }
            continue;
        }

        if in_code_block {
            code_block_text.push_str(line);
            code_block_text.push('\n');
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            html.push_str("<br>");
            continue;
        }

        if trimmed == "---" || trimmed == "___" || trimmed == "***" {
            html.push_str(&format!(
                "<hr style='border: 0; border-top: 1px solid {code_border}; margin: 4px 0;'>"
            ));
            continue;
        }

        let (is_header, content) = if let Some(h) = trimmed.strip_prefix("### ") {
            (true, h)
        } else if let Some(h) = trimmed.strip_prefix("## ") {
            (true, h)
        } else if let Some(h) = trimmed.strip_prefix("# ") {
            (true, h)
        } else {
            (false, trimmed)
        };

        let mut escaped = html_escape(content);

        // Replace `inline code`
        static INLINE_CODE_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let code_re =
            INLINE_CODE_RE.get_or_init(|| regex::Regex::new(r"`([^`]+)`").expect("regex"));
        let rep_code = format!(
            "<code style='background-color: {code_bg}; border: 1px solid {code_border}; border-radius: 3px; padding: 1px 3px; font-family: monospace; color: {code_color};'>$1</code>"
        );
        escaped = code_re
            .replace_all(&escaped, rep_code.as_str())
            .into_owned();

        // Replace **bold**
        static BOLD_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let bold_re = BOLD_RE.get_or_init(|| regex::Regex::new(r"\*\*([^*]+)\*\*").expect("regex"));
        escaped = bold_re.replace_all(&escaped, "<b>$1</b>").into_owned();

        if is_header {
            html.push_str(&format!(
                "<div style='color: {text_color}; font-weight: bold;'>{escaped}</div>"
            ));
        } else {
            html.push_str(&format!(
                "<div style='color: {text_color};'>{escaped}</div>"
            ));
        }
    }

    if in_code_block && !code_block_text.trim().is_empty() {
        let escaped_code = html_escape(code_block_text.trim());
        html.push_str(&format!(
            "<pre style='margin: 4px 0; padding: 4px 6px; background-color: {code_bg}; border: 1px solid {code_border}; border-radius: 4px; font-family: monospace; color: {code_color};'><code>{escaped_code}</code></pre>"
        ));
    }

    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_framing() {
        let msg = json!({"jsonrpc":"2.0","id":1,"method":"initialize"});
        let bytes = encode_message(&msg);
        let header = format!(
            "Content-Length: {}\r\n\r\n",
            serde_json::to_vec(&msg).unwrap().len()
        );
        assert!(bytes.starts_with(header.as_bytes()));
        let mut buf = bytes;
        buf.extend_from_slice(b"trailing");
        let out = decode_messages(&mut buf);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["method"], "initialize");
        assert_eq!(buf, b"trailing");
    }

    #[test]
    fn language_from_path() {
        assert_eq!(language_id_for_path("a.cpp"), Some("cpp"));
        assert_eq!(language_id_for_path("a.ino"), Some("cpp"));
        assert_eq!(language_id_for_path("a.c"), Some("c"));
        assert_eq!(language_id_for_path("a.asm"), None);
    }

    #[test]
    fn hover_markdown() {
        let v = json!({"language":"cpp","value":"int x"});
        assert!(parse_hover(&v).contains("int x"));
    }

    #[test]
    fn hover_markdown_formatting() {
        let md = "### function `delay`\n---\n```cpp\nvoid delay(unsigned long ms)\n```\nPauses the program for **ms** milliseconds.";
        let html_light = format_hover_markdown(md, false);
        assert!(html_light.contains("<pre style='margin: 4px 0;"));
        assert!(html_light.contains("<code>void delay(unsigned long ms)</code>"));
        assert!(html_light.contains("<hr style='border: 0;"));
        assert!(html_light.contains("<b>ms</b>"));
        assert!(html_light.contains("<code style='background-color:"));

        let html_dark = format_hover_markdown(md, true);
        assert!(html_dark.contains("<pre style='margin: 4px 0;"));
        assert!(html_dark.contains("#1e1e24")); // Dark code background
    }

    #[test]
    fn signature_html_basic() {
        let sig = SignatureHelp {
            label: "void foo(int a, float b, char c)".into(),
            active_parameter: 1,
            parameters: vec!["int a".into(), "float b".into(), "char c".into()],
        };
        let html = format_signature_html(&sig);
        assert!(html.contains("<b>"), "active param should be bold");
        // "float b" should be bold (active_parameter = 1)
        assert!(html.contains("<b>float b</b>"), "second param bold: {html}");
        // Others should not be bold
        assert!(!html.contains("<b>int a</b>"));
    }

    #[test]
    fn signature_html_no_parens() {
        let sig = SignatureHelp {
            label: "some_macro".into(),
            active_parameter: 0,
            parameters: vec![],
        };
        let html = format_signature_html(&sig);
        assert!(html.contains("some_macro"));
    }

    #[test]
    fn signature_html_nested_template() {
        let sig = SignatureHelp {
            label: "void bar(std::vector<int> v, int n)".into(),
            active_parameter: 0,
            parameters: vec!["std::vector<int> v".into(), "int n".into()],
        };
        let html = format_signature_html(&sig);
        // The <int> template should not break the comma splitting
        assert!(html.contains("<b>"));
    }

    #[test]
    fn signature_html_empty() {
        let sig = SignatureHelp::default();
        assert!(format_signature_html(&sig).is_empty());
    }
}
