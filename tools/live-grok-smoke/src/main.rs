//! Manual, opt-in live validation harness for stock Grok ACP stdio.
//!
//! Classification: **manual local / live authenticated smoke**
//! Never part of standard CI. Never stores credentials.
//!
//! # Opt-in
//!
//! ```text
//! # Dry-run (safe; no process launch, no provider usage)
//! cargo run -p live-grok-smoke -- dry-run
//!
//! # Discover binary only
//! cargo run -p live-grok-smoke -- discover
//!
//! # Live stages require explicit env + subcommand:
//! #   TRACER_LIVE_GROK=1  (or TRACER_LIVE_SMOKE=1)
//! cargo run -p live-grok-smoke -- run
//! cargo run -p live-grok-smoke -- run --through initialize
//! cargo run -p live-grok-smoke -- run --scenarios LVS-01,LVS-02
//! ```
//!
//! Credentials must already exist in the operator environment (e.g. logged-in
//! grok.com session under GROK_HOME, or provider env the stock binary reads).
//! This tool never prints tokens and never writes secrets to evidence files.

mod approval;
mod discovery;
mod evidence;
mod sanitize;
mod stages;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use evidence::{EvidenceReport, LiveClassification};
use stages::{run_dry_run, run_live, RunConfig, StageId};

fn usage() -> &'static str {
    r#"live-grok-smoke — manual opt-in stock Grok ACP smoke + approval harness

USAGE:
  live-grok-smoke <COMMAND> [OPTIONS]

COMMANDS:
  dry-run           Validate LVS spawn config / stage plan without launching Grok
  discover          Locate grok binary + report platform/version (no ACP session)
  run               Execute live LVS stages (requires TRACER_LIVE_GROK=1)
  approval-dry-run  Validate LVA approval suite plan without launching Grok (W2-D)
  approval-run      Live LVA-01..LVA-07 approval reverse-request suite
                    (requires TRACER_LIVE_GROK=1)
  help              Show this help

OPTIONS (run / dry-run / approval-*):
  --through <stage>     Stop after stage (LVS only; discovery|startup|initialize|
                        auth_requirement|session|prompt|stream|
                        approval|cancel|shutdown). Default: full plan.
  --scenarios <list>    Comma-separated LVS-01..LVS-08 or LVA-01..LVA-07
  --cwd <path>          Project cwd for session/new (default: repo root guess)
  --grok <path>         Explicit grok executable (else TRACER_GROK_BIN / PATH)
  --prompt <text>       Public-safe prompt text (LVA default induces tool use)
  --out <path>          Write sanitized JSON evidence to path
  --allow-unauth        Continue through auth-gated stages as BLOCKED_BY_AUTH
                        without treating the overall run as FAIL

ENV:
  TRACER_LIVE_GROK=1    Required for `run` / `approval-run`
  TRACER_LIVE_SMOKE=1   Accepted alias of TRACER_LIVE_GROK
  TRACER_GROK_BIN       Optional absolute/relative path to grok
  GROK_HOME             Optional hermetic Grok home (recommended for probes)

SAFETY:
  - Never part of standard CI
  - Never commits credentials or private prompts
  - Evidence is sanitized (tokens redacted)
  - Dry-run / approval-dry-run never spawn Grok agent stdio
  - Never auto-approve without explicit LVA scenario action
  - Never claim LVA PASS without observed approval.requested
"#
}

#[derive(Debug, Clone)]
struct Cli {
    command: Command,
    through: Option<StageId>,
    scenarios: Option<Vec<String>>,
    cwd: Option<PathBuf>,
    grok: Option<PathBuf>,
    prompt: Option<String>,
    out: Option<PathBuf>,
    allow_unauth: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    DryRun,
    Discover,
    Run,
    ApprovalDryRun,
    ApprovalRun,
    Help,
}

fn main() -> ExitCode {
    match parse_args(env::args().skip(1).collect()) {
        Ok(cli) => match execute(cli) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(2)
            }
        },
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!();
            eprintln!("{}", usage());
            ExitCode::from(2)
        }
    }
}

fn parse_args(args: Vec<String>) -> Result<Cli, String> {
    if args.is_empty() {
        return Ok(Cli {
            command: Command::Help,
            through: None,
            scenarios: None,
            cwd: None,
            grok: None,
            prompt: None,
            out: None,
            allow_unauth: false,
        });
    }

    let mut iter = args.into_iter();
    let cmd = match iter.next().as_deref() {
        Some("dry-run") => Command::DryRun,
        Some("discover") => Command::Discover,
        Some("run") => Command::Run,
        Some("approval-dry-run") => Command::ApprovalDryRun,
        Some("approval-run") => Command::ApprovalRun,
        Some("help") | Some("-h") | Some("--help") => Command::Help,
        Some(other) => return Err(format!("unknown command '{other}'")),
        None => Command::Help,
    };

    let mut through = None;
    let mut scenarios = None;
    let mut cwd = None;
    let mut grok = None;
    let mut prompt = None;
    let mut out = None;
    let mut allow_unauth = false;

    while let Some(a) = iter.next() {
        match a.as_str() {
            "--through" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "--through requires a stage name".to_string())?;
                through = Some(StageId::parse(&v)?);
            }
            "--scenarios" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "--scenarios requires a list".to_string())?;
                scenarios = Some(
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                );
            }
            "--cwd" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "--cwd requires a path".to_string())?;
                cwd = Some(PathBuf::from(v));
            }
            "--grok" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "--grok requires a path".to_string())?;
                grok = Some(PathBuf::from(v));
            }
            "--prompt" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "--prompt requires text".to_string())?;
                prompt = Some(v);
            }
            "--out" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "--out requires a path".to_string())?;
                out = Some(PathBuf::from(v));
            }
            "--allow-unauth" => allow_unauth = true,
            other => return Err(format!("unknown option '{other}'")),
        }
    }

    Ok(Cli {
        command: cmd,
        through,
        scenarios,
        cwd,
        grok,
        prompt,
        out,
        allow_unauth,
    })
}

fn live_opt_in() -> bool {
    env_truthy("TRACER_LIVE_GROK") || env_truthy("TRACER_LIVE_SMOKE")
}

fn env_truthy(key: &str) -> bool {
    match env::var(key) {
        Ok(v) => {
            let t = v.trim();
            t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

fn execute(cli: Cli) -> Result<ExitCode, String> {
    match cli.command {
        Command::Help => {
            println!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        Command::DryRun => {
            let cfg = build_config(&cli, /*live*/ false)?;
            let report = run_dry_run(&cfg)?;
            print_and_write(&report, cli.out.as_ref())?;
            Ok(exit_for_classification(report.classification))
        }
        Command::Discover => {
            let cfg = build_config(&cli, /*live*/ false)?;
            let report = stages::run_discover_only(&cfg)?;
            print_and_write(&report, cli.out.as_ref())?;
            Ok(exit_for_classification(report.classification))
        }
        Command::Run => {
            if !live_opt_in() {
                return Err(
                    "live `run` requires TRACER_LIVE_GROK=1 (or TRACER_LIVE_SMOKE=1). \
                     Use `dry-run` or `discover` without the flag."
                        .into(),
                );
            }
            eprintln!(
                "live-grok-smoke: TRACER_LIVE_GROK opt-in detected — \
                 may spawn stock Grok and consume provider usage"
            );
            let cfg = build_config(&cli, /*live*/ true)?;
            let report = run_live(&cfg)?;
            print_and_write(&report, cli.out.as_ref())?;
            Ok(exit_for_classification(report.classification))
        }
        Command::ApprovalDryRun => {
            let cfg = build_config(&cli, /*live*/ false)?;
            let report = approval::run_approval_dry_run(&cfg)?;
            print_and_write(&report, cli.out.as_ref())?;
            Ok(exit_for_classification(report.classification))
        }
        Command::ApprovalRun => {
            if !live_opt_in() {
                return Err(
                    "live `approval-run` requires TRACER_LIVE_GROK=1 (or TRACER_LIVE_SMOKE=1). \
                     Use `approval-dry-run` without the flag."
                        .into(),
                );
            }
            eprintln!(
                "live-grok-smoke: TRACER_LIVE_GROK opt-in detected — approval suite may \
                 spawn stock Grok, induce tool permission reverse-requests, and consume \
                 provider usage. Never auto-approves without LVA scenario action."
            );
            let cfg = build_config(&cli, /*live*/ true)?;
            let report = approval::run_approval_live(&cfg)?;
            print_and_write(&report, cli.out.as_ref())?;
            Ok(exit_for_classification(report.classification))
        }
    }
}

fn build_config(cli: &Cli, live: bool) -> Result<RunConfig, String> {
    let cwd = cli
        .cwd
        .clone()
        .or_else(guess_repo_root)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let prompt = cli
        .prompt
        .clone()
        .unwrap_or_else(|| stages::DEFAULT_PUBLIC_PROMPT.to_string());

    // Reject obviously private-looking env-sourced prompts; operators must pass
    // public-safe text via --prompt.
    if prompt_looks_sensitive(&prompt) {
        return Err(
            "refusing prompt that appears to contain secrets (token/key/password patterns). \
             Pass a public-safe --prompt."
                .into(),
        );
    }

    Ok(RunConfig {
        live,
        through: cli.through,
        scenarios: cli.scenarios.clone(),
        cwd,
        grok_override: cli
            .grok
            .clone()
            .or_else(|| env::var_os("TRACER_GROK_BIN").map(PathBuf::from)),
        prompt,
        allow_unauth: cli.allow_unauth,
    })
}

fn prompt_looks_sensitive(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    const NEEDLES: &[&str] = &[
        "api_key",
        "apikey",
        "authorization:",
        "bearer ",
        "x-api-key",
        "password=",
        "secret=",
        "private_key",
    ];
    NEEDLES.iter().any(|n| lower.contains(n))
}

fn guess_repo_root() -> Option<PathBuf> {
    // When run via cargo -p live-grok-smoke, CARGO_MANIFEST_DIR is tools/live-grok-smoke
    if let Ok(m) = env::var("CARGO_MANIFEST_DIR") {
        let mut p = PathBuf::from(m);
        // tools/live-grok-smoke → repo root
        if p.ends_with("live-grok-smoke") {
            p.pop();
            if p.ends_with("tools") {
                p.pop();
                return Some(p);
            }
        }
    }
    // Walk up from cwd looking for Cargo.toml workspace
    let mut dir = env::current_dir().ok()?;
    for _ in 0..8 {
        let cargo = dir.join("Cargo.toml");
        if cargo.is_file() {
            if let Ok(txt) = std::fs::read_to_string(&cargo) {
                if txt.contains("[workspace]") {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn print_and_write(report: &EvidenceReport, out: Option<&PathBuf>) -> Result<(), String> {
    let json =
        serde_json::to_string_pretty(report).map_err(|e| format!("serialize evidence: {e}"))?;
    // Always print sanitized report to stdout for operators / capture.
    println!("{json}");
    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| format!("create evidence dir: {e}"))?;
            }
        }
        std::fs::write(path, &json).map_err(|e| format!("write evidence: {e}"))?;
        eprintln!("wrote sanitized evidence: {}", path.display());
    }
    Ok(())
}

fn exit_for_classification(c: LiveClassification) -> ExitCode {
    match c {
        LiveClassification::Pass
        | LiveClassification::NotRun
        | LiveClassification::Partial
        | LiveClassification::NotObserved
        | LiveClassification::UnsupportedByPrompt => ExitCode::SUCCESS,
        // BLOCKED_BY_AUTH is not a product failure — operator must authenticate.
        LiveClassification::BlockedByAuth => ExitCode::SUCCESS,
        LiveClassification::Fail => ExitCode::from(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dry_run() {
        let cli = parse_args(vec![
            "dry-run".into(),
            "--through".into(),
            "initialize".into(),
        ])
        .expect("parse");
        assert_eq!(cli.command, Command::DryRun);
        assert_eq!(cli.through, Some(StageId::Initialize));
    }

    #[test]
    fn parses_run_scenarios() {
        let cli = parse_args(vec![
            "run".into(),
            "--scenarios".into(),
            "LVS-01,LVS-02".into(),
            "--allow-unauth".into(),
        ])
        .expect("parse");
        assert_eq!(cli.command, Command::Run);
        assert!(cli.allow_unauth);
        assert_eq!(
            cli.scenarios.as_ref().unwrap(),
            &vec!["LVS-01".to_string(), "LVS-02".to_string()]
        );
    }

    #[test]
    fn parses_approval_commands() {
        let dry = parse_args(vec!["approval-dry-run".into()]).expect("parse");
        assert_eq!(dry.command, Command::ApprovalDryRun);
        let live = parse_args(vec![
            "approval-run".into(),
            "--scenarios".into(),
            "LVA-01,LVA-04".into(),
        ])
        .expect("parse");
        assert_eq!(live.command, Command::ApprovalRun);
        assert_eq!(
            live.scenarios.as_ref().unwrap(),
            &vec!["LVA-01".to_string(), "LVA-04".to_string()]
        );
    }

    #[test]
    fn sensitive_prompt_rejected() {
        assert!(prompt_looks_sensitive("here is my api_key=sk-test"));
        assert!(!prompt_looks_sensitive(stages::DEFAULT_PUBLIC_PROMPT));
    }

    #[test]
    fn classification_exit_codes() {
        assert_eq!(
            exit_for_classification(LiveClassification::BlockedByAuth),
            ExitCode::SUCCESS
        );
        assert_eq!(
            exit_for_classification(LiveClassification::NotObserved),
            ExitCode::SUCCESS
        );
        assert_eq!(
            exit_for_classification(LiveClassification::UnsupportedByPrompt),
            ExitCode::SUCCESS
        );
        assert_eq!(
            exit_for_classification(LiveClassification::Fail),
            ExitCode::from(1)
        );
    }

    #[test]
    fn dry_run_produces_not_run_or_pass_structure() {
        let cfg = RunConfig {
            live: false,
            through: None,
            scenarios: None,
            cwd: PathBuf::from("."),
            grok_override: Some(PathBuf::from("grok")),
            prompt: stages::DEFAULT_PUBLIC_PROMPT.into(),
            allow_unauth: true,
        };
        let report = run_dry_run(&cfg).expect("dry-run");
        assert_eq!(report.harness, "live-grok-smoke");
        assert_eq!(
            report.classification_tier,
            "manual_local_live_authenticated_smoke"
        );
        assert!(!report.stages.is_empty());
        // Dry-run never launches; overall is NotRun (plan validated) or Pass for construction.
        assert!(matches!(
            report.classification,
            LiveClassification::NotRun | LiveClassification::Pass
        ));
        // No stage should claim live process spawn.
        for s in &report.stages {
            assert!(
                s.notes
                    .iter()
                    .any(|n| n.contains("dry-run") || n.contains("not launched"))
                    || s.status == evidence::StageStatus::Skipped
                    || s.status == evidence::StageStatus::Pass
                    || s.status == evidence::StageStatus::NotRun,
                "unexpected dry-run stage: {:?}",
                s
            );
        }
    }
}
