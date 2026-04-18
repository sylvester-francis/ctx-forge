//! Per-ecosystem stdlib blocklists. Imports matching these names are
//! skipped during scan — they're not user-installable deps, they ship
//! with the language runtime.

use crate::source::Ecosystem;

/// Rust pseudo-crates — stdlib + the scope-local keywords.
const RUST_STDLIB: &[&str] = &[
    "std",
    "core",
    "alloc",
    "crate",
    "self",
    "super",
    "test",
    "proc_macro",
];

/// Node.js builtins (both bare and `node:`-prefixed forms handled by caller).
const JS_BUILTINS: &[&str] = &[
    "assert",
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "sys",
    "timers",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "wasi",
    "worker_threads",
    "zlib",
];

/// Python stdlib (curated subset covering the most-imported modules).
const PYTHON_STDLIB: &[&str] = &[
    "__future__",
    "abc",
    "argparse",
    "array",
    "ast",
    "asyncio",
    "base64",
    "binascii",
    "bisect",
    "builtins",
    "bz2",
    "calendar",
    "cgi",
    "collections",
    "concurrent",
    "contextlib",
    "copy",
    "csv",
    "ctypes",
    "dataclasses",
    "datetime",
    "decimal",
    "difflib",
    "dis",
    "doctest",
    "email",
    "enum",
    "errno",
    "fcntl",
    "filecmp",
    "fileinput",
    "fnmatch",
    "fractions",
    "functools",
    "gc",
    "getopt",
    "getpass",
    "gettext",
    "glob",
    "gzip",
    "hashlib",
    "heapq",
    "hmac",
    "html",
    "http",
    "importlib",
    "inspect",
    "io",
    "ipaddress",
    "itertools",
    "json",
    "keyword",
    "linecache",
    "locale",
    "logging",
    "lzma",
    "math",
    "mimetypes",
    "multiprocessing",
    "numbers",
    "operator",
    "os",
    "pathlib",
    "pickle",
    "pkgutil",
    "platform",
    "plistlib",
    "pprint",
    "profile",
    "pstats",
    "queue",
    "random",
    "re",
    "secrets",
    "select",
    "selectors",
    "shelve",
    "shlex",
    "shutil",
    "signal",
    "site",
    "smtplib",
    "socket",
    "socketserver",
    "sqlite3",
    "ssl",
    "stat",
    "string",
    "struct",
    "subprocess",
    "sys",
    "sysconfig",
    "tarfile",
    "tempfile",
    "textwrap",
    "threading",
    "time",
    "timeit",
    "tkinter",
    "token",
    "tokenize",
    "tomllib",
    "trace",
    "traceback",
    "types",
    "typing",
    "typing_extensions",
    "unicodedata",
    "unittest",
    "urllib",
    "uuid",
    "venv",
    "warnings",
    "weakref",
    "webbrowser",
    "xml",
    "xmlrpc",
    "zipfile",
    "zipimport",
    "zlib",
    "zoneinfo",
];

/// Go stdlib detection: a Go import path is stdlib iff its first path
/// segment contains no `.` (third-party paths start with a domain like
/// `github.com`, `gopkg.in`, `golang.org/x`).
fn is_go_stdlib(path: &str) -> bool {
    let first = path.split('/').next().unwrap_or("");
    !first.contains('.')
}

/// Is the raw import name part of the ecosystem's stdlib?
pub fn is_stdlib(name: &str, ecosystem: Ecosystem) -> bool {
    match ecosystem {
        Ecosystem::Rust => RUST_STDLIB.contains(&name),
        Ecosystem::Js => {
            let stripped = name.strip_prefix("node:").unwrap_or(name);
            JS_BUILTINS.contains(&stripped)
        }
        Ecosystem::Python => PYTHON_STDLIB.contains(&name),
        Ecosystem::Go => is_go_stdlib(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_stdlib_is_blocked() {
        assert!(is_stdlib("std", Ecosystem::Rust));
        assert!(is_stdlib("core", Ecosystem::Rust));
        assert!(is_stdlib("crate", Ecosystem::Rust));
        assert!(!is_stdlib("tokio", Ecosystem::Rust));
        assert!(!is_stdlib("serde_json", Ecosystem::Rust));
    }

    #[test]
    fn js_builtins_with_and_without_node_prefix() {
        assert!(is_stdlib("fs", Ecosystem::Js));
        assert!(is_stdlib("node:fs", Ecosystem::Js));
        assert!(is_stdlib("crypto", Ecosystem::Js));
        assert!(!is_stdlib("react", Ecosystem::Js));
        assert!(!is_stdlib("@next/core", Ecosystem::Js));
    }

    #[test]
    fn python_stdlib_covers_common_modules() {
        for m in ["os", "sys", "json", "re", "typing", "asyncio", "pathlib"] {
            assert!(is_stdlib(m, Ecosystem::Python), "{m} should be stdlib");
        }
        assert!(!is_stdlib("requests", Ecosystem::Python));
        assert!(!is_stdlib("fastapi", Ecosystem::Python));
    }

    #[test]
    fn go_stdlib_has_no_dot_in_first_segment() {
        assert!(is_stdlib("fmt", Ecosystem::Go));
        assert!(is_stdlib("net/http", Ecosystem::Go));
        assert!(is_stdlib("encoding/json", Ecosystem::Go));
        assert!(!is_stdlib("github.com/gin-gonic/gin", Ecosystem::Go));
        assert!(!is_stdlib("golang.org/x/sync", Ecosystem::Go));
    }
}
