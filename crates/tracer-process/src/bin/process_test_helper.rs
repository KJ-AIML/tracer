//! Deterministic fake process for tracer-process tests.
//!
//! Subcommands:
//! - `echo-stdout <text>`
//! - `echo-stderr <text>`
//! - `echo-both <stdout_text> <stderr_text>`
//! - `sleep-ms <n>`
//! - `exit <code>`
//! - `hang-until-stdin-close` — read stdin until EOF, then exit 0
//! - `spawn-child-sleep-ms <n>` — spawn a grandchild that sleeps, print both pids, then sleep
//! - `read-stdin-line` — read one line from stdin, echo to stdout, exit

use std::io::{self, BufRead, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn main() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!("usage: tracer-process-test-helper <subcommand> ...");
        std::process::exit(2);
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "echo-stdout" => {
            let text = args.first().map(String::as_str).unwrap_or("");
            print!("{text}");
            let _ = io::stdout().flush();
        }
        "echo-stderr" => {
            let text = args.first().map(String::as_str).unwrap_or("");
            eprint!("{text}");
            let _ = io::stderr().flush();
        }
        "echo-both" => {
            let out = args.first().map(String::as_str).unwrap_or("");
            let err = args.get(1).map(String::as_str).unwrap_or("");
            print!("{out}");
            let _ = io::stdout().flush();
            eprint!("{err}");
            let _ = io::stderr().flush();
        }
        "sleep-ms" => {
            let ms: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(1000);
            thread::sleep(Duration::from_millis(ms));
        }
        "exit" => {
            let code: i32 = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            std::process::exit(code);
        }
        "hang-until-stdin-close" => {
            let mut sink = Vec::new();
            let _ = io::stdin().read_to_end(&mut sink);
        }
        "read-stdin-line" => {
            let mut line = String::new();
            let _ = io::stdin().lock().read_line(&mut line);
            print!("{line}");
            let _ = io::stdout().flush();
        }
        "spawn-child-sleep-ms" => {
            // Spawn a long-lived grandchild, print parent+child pids, then sleep.
            // Used to verify Job Object / process-group tree kill (no orphans).
            let ms: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(60_000);
            let self_exe = std::env::current_exe().expect("current_exe");
            let mut child = Command::new(&self_exe)
                .arg("sleep-ms")
                .arg(ms.to_string())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn grandchild");
            let child_pid = child.id();
            let parent_pid = std::process::id();
            println!("parent_pid={parent_pid}");
            println!("child_pid={child_pid}");
            let _ = io::stdout().flush();
            // Stay alive so the manager owns this process; grandchild is in tree.
            thread::sleep(Duration::from_millis(ms));
            let _ = child.kill();
            let _ = child.wait();
        }
        other => {
            eprintln!("unknown subcommand: {other}");
            std::process::exit(2);
        }
    }
}
