//! UI-agnostic composer contract shared by the terminal UI and the desktop
//! bridge. Parsing, the command catalog, and completion live here so every
//! surface accepts exactly the same slash commands, aliases, and arguments;
//! each UI only maps [`Surface`] results onto its own panels.

pub mod completion;

use crate::domain::event::{ApprovalDecision, MessageTarget, RunId, RunStepStatus, UiCommand};
use crate::domain::mode::TerminalMode;
use crate::domain::team::MemberId;

/// A panel or drawer a submission can open. UI shells map each variant onto
/// their own presentation (a TUI drawer, a desktop panel, …).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Surface {
    /// Team roster, sessions, and approval settings.
    Team,
    /// Run status, steps, and events.
    Runs,
    /// Raw runtime logs.
    Logs,
    /// Working-tree git diff.
    Diff,
    /// Collaboration mode panel (bindings, overrides, run launcher).
    Mode,
    /// One member's focused log stream.
    MemberLogs(MemberId),
}

/// One entry of the composer command catalog: the canonical name, a short
/// hint, and whether the command takes an argument. Also drives `/help`
/// palettes and GUI command palettes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub name: &'static str,
    pub hint: &'static str,
    pub takes_argument: bool,
}

/// The canonical command catalog. Order is presentation order; keep entries in
/// sync with `docs/commands.md` (enforced by a test).
pub const COMMAND_CATALOG: &[CommandSpec] = &[
    CommandSpec {
        name: "ask",
        hint: "send to one member",
        takes_argument: true,
    },
    CommandSpec {
        name: "all",
        hint: "send to everyone",
        takes_argument: true,
    },
    CommandSpec {
        name: "attach",
        hint: "open a member's native CLI session",
        takes_argument: true,
    },
    CommandSpec {
        name: "approve",
        hint: "approve first pending",
        takes_argument: false,
    },
    CommandSpec {
        name: "block",
        hint: "mark a run blocked",
        takes_argument: true,
    },
    CommandSpec {
        name: "continue",
        hint: "resume latest or selected run",
        takes_argument: true,
    },
    CommandSpec {
        name: "diff",
        hint: "show working-tree git diff",
        takes_argument: false,
    },
    CommandSpec {
        name: "exit",
        hint: "exit Asterline",
        takes_argument: false,
    },
    CommandSpec {
        name: "export",
        hint: "export session to Claude Code format",
        takes_argument: true,
    },
    CommandSpec {
        name: "find",
        hint: "search the transcript",
        takes_argument: true,
    },
    CommandSpec {
        name: "focus",
        hint: "view a member's logs",
        takes_argument: true,
    },
    CommandSpec {
        name: "help",
        hint: "show commands",
        takes_argument: false,
    },
    CommandSpec {
        name: "import",
        hint: "import native session transcript into chat",
        takes_argument: true,
    },
    CommandSpec {
        name: "logs",
        hint: "raw logs · stderr · warnings",
        takes_argument: false,
    },
    CommandSpec {
        name: "mode",
        hint: "open mode panel or switch dispatch mode",
        takes_argument: true,
    },
    CommandSpec {
        name: "new",
        hint: "start a fresh chat (new session, cleared transcript)",
        takes_argument: false,
    },
    CommandSpec {
        name: "note",
        hint: "record a run checkpoint",
        takes_argument: true,
    },
    CommandSpec {
        name: "reject",
        hint: "reject first pending",
        takes_argument: false,
    },
    CommandSpec {
        name: "resume",
        hint: "choose and restore a saved chat",
        takes_argument: false,
    },
    CommandSpec {
        name: "retry",
        hint: "re-send the latest user request",
        takes_argument: false,
    },
    CommandSpec {
        name: "runs",
        hint: "run status · next action",
        takes_argument: false,
    },
    CommandSpec {
        name: "step",
        hint: "manage run checklist",
        takes_argument: true,
    },
    CommandSpec {
        name: "team",
        hint: "edit roster · sessions · approvals",
        takes_argument: false,
    },
    CommandSpec {
        name: "verify",
        hint: "verify latest or selected run",
        takes_argument: true,
    },
];

/// Dispatch modes offered after `/mode `, with their hints.
pub const MODE_OPTIONS: &[(&str, &str)] = &[
    ("normal", "keep using direct messages until changed"),
    ("review", "keep using builder/reviewer runs until changed"),
    ("plan", "keep using leader/checklist runs until changed"),
    (
        "brainstorm",
        "keep using multi-wave idea generation until changed",
    ),
    (
        "team",
        "keep using coordinator-driven team runs until changed",
    ),
];

/// What submitting the composer should do, independent of any UI shell.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::large_enum_variant)] // Runtime commands stay unboxed at the UI boundary.
pub enum ParsedInput {
    /// Exit the UI shell and begin normal runtime shutdown.
    Exit,
    /// Open one member's native interactive CLI session.
    Attach { member: MemberId },
    /// A targeted slash invocation resolved only against the target backend's
    /// discovered prompt-invocable skills before any prompt is sent to a
    /// noninteractive backend runner.
    TargetedSlash { member: MemberId, body: String },
    /// Send a command to the runtime.
    Runtime(UiCommand),
    /// Open a UI surface (a local UI action).
    Surface(Surface),
    /// Approve (true) or reject (false) the first pending approval.
    ApproveFirst(ApprovalDecision),
    /// Search the transcript (`/find`); empty query clears the search.
    FindInChat(String),
    /// Show help.
    Help,
    /// Reject invalid command syntax while leaving the draft untouched.
    Invalid(String),
    /// Non-empty message text without an explicit target prefix.
    NeedsTarget,
    /// Nothing to do (blank input).
    Empty,
}

/// Parse the composer text.
pub fn parse(input: &str) -> ParsedInput {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return ParsedInput::Empty;
    }

    if let Some(rest) = trimmed.strip_prefix('/') {
        return parse_slash(rest);
    }
    if let Some(rest) = trimmed.strip_prefix('@') {
        let (member, body) = split_first_word(rest);
        if member.is_empty() || body.is_empty() {
            return ParsedInput::Empty;
        }
        if let Some(input) = parse_targeted_slash(member, body) {
            return input;
        }
        let target = if member == "all" {
            MessageTarget::All
        } else {
            MessageTarget::Member(MemberId::new(member))
        };
        return ParsedInput::Runtime(UiCommand::UserMessage {
            target,
            body: trimmed.to_string(),
        });
    }

    ParsedInput::NeedsTarget
}

/// `@member` / `@all` / `/ask member` / `/all` typed with no message body.
/// Lets an image-only send keep an explicit target.
pub fn parse_target_only(input: &str) -> Option<MessageTarget> {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix('@') {
        let (member, body) = split_first_word(rest);
        if !member.is_empty() && body.is_empty() {
            return Some(target_from_member_token(member));
        }
    }
    if let Some(rest) = trimmed.strip_prefix("/ask") {
        let (member, body) = split_first_word(rest);
        if !member.is_empty() && body.is_empty() {
            return Some(target_from_member_token(member));
        }
    }
    if trimmed == "/all" {
        return Some(MessageTarget::All);
    }
    None
}

fn target_from_member_token(member: &str) -> MessageTarget {
    if member == "all" {
        MessageTarget::All
    } else {
        MessageTarget::Member(MemberId::new(member))
    }
}

fn parse_slash(rest: &str) -> ParsedInput {
    let (cmd, arg) = split_first_word(rest);
    match cmd {
        "ask" => {
            let (member, body) = split_first_word(arg);
            if member.is_empty() || body.is_empty() {
                ParsedInput::Help
            } else if let Some(input) = parse_targeted_slash(member, body) {
                input
            } else {
                let target = if member == "all" {
                    MessageTarget::All
                } else {
                    MessageTarget::Member(MemberId::new(member))
                };
                ParsedInput::Runtime(UiCommand::UserMessage {
                    target,
                    body: format!("@{} {}", member, body),
                })
            }
        }
        "all" => {
            if arg.is_empty() {
                ParsedInput::Help
            } else if arg.trim_start().starts_with('/') {
                ParsedInput::Invalid(
                    "slash commands need one member; use @member /command (draft kept)".to_string(),
                )
            } else {
                ParsedInput::Runtime(UiCommand::UserMessage {
                    target: MessageTarget::All,
                    body: format!("@all {}", arg),
                })
            }
        }
        "team" if arg.is_empty() => ParsedInput::Surface(Surface::Team),
        "runs" if arg.is_empty() => ParsedInput::Surface(Surface::Runs),
        "logs" if arg.is_empty() => ParsedInput::Surface(Surface::Logs),
        "diff" if arg.is_empty() => ParsedInput::Surface(Surface::Diff),
        "attach" => {
            let (member, extra) = split_first_word(arg);
            if member.is_empty() {
                ParsedInput::Help
            } else if member == "all" {
                ParsedInput::Invalid("/attach needs one member; use /attach <member>".to_string())
            } else if !extra.is_empty() {
                ParsedInput::Invalid("/attach does not accept arguments; draft kept".to_string())
            } else {
                ParsedInput::Attach {
                    member: MemberId::new(member),
                }
            }
        }
        // Both spellings intentionally perform the same durable reset. Keep
        // `/new` for muscle memory and accept `/clear` when it is submitted
        // directly (rather than only after completion rewrites it).
        "new" | "clear" if arg.is_empty() => ParsedInput::Runtime(UiCommand::NewSession),
        "resume" if arg.is_empty() => ParsedInput::Runtime(UiCommand::RequestResume),
        "exit" if arg.is_empty() => ParsedInput::Exit,
        "retry" if arg.is_empty() => ParsedInput::Runtime(UiCommand::Retry),
        "approve" if arg.is_empty() => ParsedInput::ApproveFirst(ApprovalDecision::Approve),
        "reject" if arg.is_empty() => ParsedInput::ApproveFirst(ApprovalDecision::Reject),
        "mode" if arg.is_empty() => ParsedInput::Surface(Surface::Mode),
        "mode" => parse_mode_selector(arg),
        "find" => ParsedInput::FindInChat(arg.to_string()),
        "continue" => {
            let (first, rest) = split_first_word(arg);
            let (run_id, note) = if let Some(run_id) = parse_run_id(first) {
                (Some(run_id), (!rest.is_empty()).then(|| rest.to_string()))
            } else {
                (None, (!arg.is_empty()).then(|| arg.to_string()))
            };
            ParsedInput::Runtime(UiCommand::ContinueRun { run_id, note })
        }
        "note" => {
            let (first, rest) = split_first_word(arg);
            let (run_id, note) = if let Some(run_id) = parse_run_id(first) {
                (Some(run_id), rest)
            } else {
                (None, arg)
            };
            if note.is_empty() {
                ParsedInput::Help
            } else {
                ParsedInput::Runtime(UiCommand::NoteRun {
                    run_id,
                    note: note.to_string(),
                })
            }
        }
        "block" => {
            let (first, rest) = split_first_word(arg);
            let (run_id, reason) = if let Some(run_id) = parse_run_id(first) {
                (Some(run_id), rest)
            } else {
                (None, arg)
            };
            if reason.is_empty() {
                ParsedInput::Help
            } else {
                ParsedInput::Runtime(UiCommand::BlockRun {
                    run_id,
                    reason: reason.to_string(),
                })
            }
        }
        "verify" => {
            let (first, rest) = split_first_word(arg);
            let (run_id, command) = if let Some(run_id) = parse_run_id(first) {
                (Some(run_id), (!rest.is_empty()).then(|| rest.to_string()))
            } else {
                (None, (!arg.is_empty()).then(|| arg.to_string()))
            };
            ParsedInput::Runtime(UiCommand::VerifyRun { run_id, command })
        }
        "step" => parse_step_command(arg),
        "focus" => {
            let (member, extra) = split_first_word(arg);
            if member.is_empty() {
                ParsedInput::Help
            } else if !extra.is_empty() {
                ParsedInput::Invalid(
                    "/focus accepts exactly one member; trailing arguments were not used; draft kept"
                        .to_string(),
                )
            } else {
                ParsedInput::Surface(Surface::MemberLogs(MemberId::new(member)))
            }
        }
        "import" => {
            let (first, rest) = split_first_word(arg);
            if first.is_empty() {
                ParsedInput::Help
            } else if rest.is_empty() {
                ParsedInput::Runtime(UiCommand::ImportSession {
                    member: None,
                    session_id: first.to_string(),
                })
            } else {
                ParsedInput::Runtime(UiCommand::ImportSession {
                    member: Some(MemberId::new(first)),
                    session_id: rest.to_string(),
                })
            }
        }
        "export" => {
            let (first, extra) = split_first_word(arg);
            if !extra.is_empty() {
                ParsedInput::Invalid(
                    "/export accepts at most one format argument (e.g. `/export claude`); draft kept"
                        .to_string(),
                )
            } else {
                ParsedInput::Runtime(UiCommand::ExportSession {
                    format: (!first.is_empty()).then(|| first.to_string()),
                })
            }
        }
        "help" if arg.is_empty() => ParsedInput::Help,
        "team" | "runs" | "logs" | "diff" | "new" | "clear" | "resume" | "exit" | "retry"
        | "approve" | "reject" | "help" => {
            ParsedInput::Invalid(format!("/{cmd} does not accept arguments; draft kept"))
        }
        _ => ParsedInput::Help,
    }
}

/// Parse a slash command aimed at one explicit member. Returning `None` means
/// `body` is ordinary prompt text, not a slash command. Both `@member …` and
/// `/ask member …` use this path so the latter cannot bypass the native-session
/// and discovered-skill safeguards.
fn parse_targeted_slash(member: &str, body: &str) -> Option<ParsedInput> {
    if !body.trim_start().starts_with('/') {
        return None;
    }
    if let Some(rest) = targeted_command_rest(body, "attach") {
        return Some(match (member, rest.is_empty()) {
            ("all", _) => {
                ParsedInput::Invalid("/attach needs one member; use @member /attach".to_string())
            }
            (_, true) => ParsedInput::Attach {
                member: MemberId::new(member),
            },
            _ => ParsedInput::Invalid("/attach does not accept arguments; draft kept".to_string()),
        });
    }
    if let Some(rest) = targeted_command_rest(body, "import") {
        return Some(match (member, rest.is_empty()) {
            ("all", _) => ParsedInput::Invalid(
                "/import needs one member; use @member /import <session_id>".to_string(),
            ),
            (_, true) => {
                ParsedInput::Invalid("use `@member /import <session_id>` (draft kept)".to_string())
            }
            _ => ParsedInput::Runtime(UiCommand::ImportSession {
                member: Some(MemberId::new(member)),
                session_id: rest.to_string(),
            }),
        });
    }
    Some(if member == "all" {
        ParsedInput::Invalid(
            "slash commands need one member; use @member /<discovered-skill> or /attach <member> (draft kept)"
                .to_string(),
        )
    } else {
        ParsedInput::TargetedSlash {
            member: MemberId::new(member),
            body: body.to_string(),
        }
    })
}

fn targeted_command_rest<'a>(body: &'a str, command: &str) -> Option<&'a str> {
    let rest = body.strip_prefix('/')?.strip_prefix(command)?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some(rest.trim())
}

fn parse_mode_selector(arg: &str) -> ParsedInput {
    let (selected, extra) = split_first_word(arg);
    if !extra.is_empty() {
        return ParsedInput::Help;
    }
    TerminalMode::parse(selected).map_or(ParsedInput::Help, |mode| {
        ParsedInput::Runtime(UiCommand::SetMode { mode })
    })
}

fn parse_step_command(arg: &str) -> ParsedInput {
    let (action, rest) = split_first_word(arg);
    match action {
        "add" => {
            let (first, rest_after_first) = split_first_word(rest);
            let (run_id, title_input) = if let Some(run_id) = parse_run_id(first) {
                (Some(run_id), rest_after_first)
            } else {
                (None, rest)
            };
            let (owner, title) = split_optional_owner(title_input);
            if title.is_empty() {
                ParsedInput::Help
            } else {
                ParsedInput::Runtime(UiCommand::AddRunStep {
                    run_id,
                    owner,
                    title: title.to_string(),
                })
            }
        }
        "todo" | "doing" | "done" | "block" | "blocked" => {
            let Some(status) = parse_run_step_status(action) else {
                return ParsedInput::Help;
            };
            let (first, rest_after_first) = split_first_word(rest);
            let (run_id, number_text, note) = if let Some(run_id) = parse_run_id(first) {
                let (number, note) = split_first_word(rest_after_first);
                (Some(run_id), number, note)
            } else {
                let (number, note) = split_first_word(rest);
                (None, number, note)
            };
            let Ok(step) = number_text.parse::<u32>() else {
                return ParsedInput::Help;
            };
            if step == 0 {
                return ParsedInput::Help;
            }
            ParsedInput::Runtime(UiCommand::UpdateRunStep {
                run_id,
                step,
                status,
                note: (!note.is_empty()).then(|| note.to_string()),
            })
        }
        "rename" | "edit" => {
            let (first, rest_after_first) = split_first_word(rest);
            let (run_id, number_text, title) = if let Some(run_id) = parse_run_id(first) {
                let (number, title) = split_first_word(rest_after_first);
                (Some(run_id), number, title)
            } else {
                let (number, title) = split_first_word(rest);
                (None, number, title)
            };
            let Ok(step) = number_text.parse::<u32>() else {
                return ParsedInput::Help;
            };
            if step == 0 || title.is_empty() {
                return ParsedInput::Help;
            }
            ParsedInput::Runtime(UiCommand::RenameRunStep {
                run_id,
                step,
                title: title.to_string(),
            })
        }
        "remove" | "delete" | "drop" => {
            let (first, rest_after_first) = split_first_word(rest);
            let (run_id, number_text, extra) = if let Some(run_id) = parse_run_id(first) {
                let (number, extra) = split_first_word(rest_after_first);
                (Some(run_id), number, extra)
            } else {
                let (number, extra) = split_first_word(rest);
                (None, number, extra)
            };
            let Ok(step) = number_text.parse::<u32>() else {
                return ParsedInput::Help;
            };
            if step == 0 {
                return ParsedInput::Help;
            }
            if !extra.is_empty() {
                return ParsedInput::Invalid(format!(
                    "/step {action} does not accept trailing arguments; draft kept"
                ));
            }
            ParsedInput::Runtime(UiCommand::RemoveRunStep { run_id, step })
        }
        "assign" | "owner" => {
            let (first, rest_after_first) = split_first_word(rest);
            let (run_id, number_text, owner_text) = if let Some(run_id) = parse_run_id(first) {
                let (number, owner) = split_first_word(rest_after_first);
                (Some(run_id), number, owner)
            } else {
                let (number, owner) = split_first_word(rest);
                (None, number, owner)
            };
            let Ok(step) = number_text.parse::<u32>() else {
                return ParsedInput::Help;
            };
            let Some(owner) = parse_owner_arg(owner_text) else {
                return ParsedInput::Help;
            };
            if step == 0 {
                return ParsedInput::Help;
            }
            ParsedInput::Runtime(UiCommand::AssignRunStep {
                run_id,
                step,
                owner: Some(owner),
            })
        }
        "unassign" | "clear-owner" | "clear_owner" => {
            let (first, rest_after_first) = split_first_word(rest);
            let (run_id, number_text, extra) = if let Some(run_id) = parse_run_id(first) {
                let (number, extra) = split_first_word(rest_after_first);
                (Some(run_id), number, extra)
            } else {
                let (number, extra) = split_first_word(rest);
                (None, number, extra)
            };
            let Ok(step) = number_text.parse::<u32>() else {
                return ParsedInput::Help;
            };
            if step == 0 {
                return ParsedInput::Help;
            }
            if !extra.is_empty() {
                return ParsedInput::Invalid(format!(
                    "/step {action} does not accept trailing arguments; draft kept"
                ));
            }
            ParsedInput::Runtime(UiCommand::AssignRunStep {
                run_id,
                step,
                owner: None,
            })
        }
        _ => ParsedInput::Help,
    }
}

fn split_optional_owner(input: &str) -> (Option<MemberId>, &str) {
    let (first, rest) = split_first_word(input);
    parse_prefixed_owner_arg(first)
        .map(|owner| (Some(owner), rest))
        .unwrap_or((None, input))
}

fn parse_prefixed_owner_arg(input: &str) -> Option<MemberId> {
    input.trim().strip_prefix('@').and_then(parse_owner_arg)
}

fn parse_owner_arg(input: &str) -> Option<MemberId> {
    let owner = input.trim().trim_start_matches('@');
    if owner.is_empty()
        || owner.eq_ignore_ascii_case("none")
        || owner.eq_ignore_ascii_case("unassigned")
        || owner.chars().any(char::is_whitespace)
    {
        None
    } else {
        Some(MemberId::new(owner))
    }
}

fn parse_run_step_status(value: &str) -> Option<RunStepStatus> {
    match value {
        "todo" => Some(RunStepStatus::Todo),
        "doing" => Some(RunStepStatus::Doing),
        "done" => Some(RunStepStatus::Done),
        "block" | "blocked" => Some(RunStepStatus::Blocked),
        _ => None,
    }
}

pub(crate) fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(idx) => (&s[..idx], s[idx..].trim()),
        None => (s, ""),
    }
}

fn parse_run_id(value: &str) -> Option<RunId> {
    let raw = value.strip_prefix("run-")?;
    raw.parse::<u64>().ok().map(RunId)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_requires_an_explicit_target_prefix() {
        assert_eq!(parse("build the parser"), ParsedInput::NeedsTarget);
    }

    #[test]
    fn target_only_accepts_member_or_all_without_a_body() {
        assert_eq!(
            parse_target_only("@builder"),
            Some(MessageTarget::Member(MemberId::new("builder")))
        );
        assert_eq!(parse_target_only("@all"), Some(MessageTarget::All));
        assert_eq!(
            parse_target_only("/ask reviewer"),
            Some(MessageTarget::Member(MemberId::new("reviewer")))
        );
        assert_eq!(parse_target_only("/all"), Some(MessageTarget::All));
        assert_eq!(parse_target_only("@builder look"), None);
        assert_eq!(parse_target_only(""), None);
    }

    #[test]
    fn at_prefix_targets_member() {
        assert_eq!(
            parse("@reviewer please check"),
            ParsedInput::Runtime(UiCommand::UserMessage {
                target: MessageTarget::Member(MemberId::new("reviewer")),
                body: "@reviewer please check".to_string(),
            })
        );
    }

    #[test]
    fn ask_command_targets_member() {
        assert_eq!(
            parse("/ask builder implement it"),
            ParsedInput::Runtime(UiCommand::UserMessage {
                target: MessageTarget::Member(MemberId::new("builder")),
                body: "@builder implement it".to_string(),
            })
        );
    }

    #[test]
    fn ask_all_command_broadcasts() {
        assert_eq!(
            parse("/ask all implement it"),
            ParsedInput::Runtime(UiCommand::UserMessage {
                target: MessageTarget::All,
                body: "@all implement it".to_string(),
            })
        );
    }

    #[test]
    fn all_command_broadcasts() {
        assert_eq!(
            parse("/all status?"),
            ParsedInput::Runtime(UiCommand::UserMessage {
                target: MessageTarget::All,
                body: "@all status?".to_string(),
            })
        );
    }

    #[test]
    fn drawer_and_control_commands() {
        assert_eq!(parse("/logs"), ParsedInput::Surface(Surface::Logs));
        assert_eq!(parse("/runs"), ParsedInput::Surface(Surface::Runs));
        assert_eq!(parse("/team"), ParsedInput::Surface(Surface::Team));
        assert_eq!(parse("/team "), ParsedInput::Surface(Surface::Team));
        assert_eq!(parse("/diff"), ParsedInput::Surface(Surface::Diff));
        assert_eq!(parse("/exit"), ParsedInput::Exit);
        assert_eq!(parse("/retry"), ParsedInput::Runtime(UiCommand::Retry));
        assert_eq!(
            parse("/approve"),
            ParsedInput::ApproveFirst(ApprovalDecision::Approve)
        );
    }

    #[test]
    fn no_argument_commands_reject_trailing_text() {
        for command in [
            "/team extra",
            "/runs extra",
            "/logs extra",
            "/diff extra",
            "/new extra",
            "/clear extra",
            "/resume extra",
            "/exit extra",
            "/retry extra",
            "/approve extra",
            "/reject extra",
            "/help extra",
        ] {
            assert!(
                matches!(parse(command), ParsedInput::Invalid(message) if message.contains("does not accept arguments")),
                "{command}"
            );
        }
    }

    #[test]
    fn fixed_arity_commands_reject_unused_trailing_text() {
        for command in [
            "/focus reviewer accidental",
            "/step remove 2 accidental",
            "/step remove run-12 2 accidental",
            "/step delete 2 accidental",
            "/step unassign 3 accidental",
            "/step clear-owner run-12 3 accidental",
        ] {
            assert!(
                matches!(parse(command), ParsedInput::Invalid(message) if message.contains("trailing")),
                "{command} must not silently discard input"
            );
        }
    }

    #[test]
    fn blank_is_empty_and_unknown_slash_is_help() {
        assert_eq!(parse("   "), ParsedInput::Empty);
        assert_eq!(parse("/wat"), ParsedInput::Help);
        assert_eq!(parse("/ask builder"), ParsedInput::Help);
    }

    #[test]
    fn model_is_not_a_composer_control() {
        assert_eq!(parse("/model"), ParsedInput::Help);
        assert_eq!(parse("/model builder gpt-5.6-sol"), ParsedInput::Help);
        assert_eq!(
            parse("@builder /model gpt-5.6-sol"),
            ParsedInput::TargetedSlash {
                member: MemberId::new("builder"),
                body: "/model gpt-5.6-sol".to_string(),
            }
        );
    }

    #[test]
    fn targeted_slashes_are_resolved_before_reaching_a_backend_prompt() {
        assert_eq!(
            parse("@builder /attach"),
            ParsedInput::Attach {
                member: MemberId::new("builder"),
            }
        );
        assert_eq!(
            parse("/attach builder"),
            ParsedInput::Attach {
                member: MemberId::new("builder"),
            }
        );
        assert_eq!(
            parse("@builder /unrecognized with args"),
            ParsedInput::TargetedSlash {
                member: MemberId::new("builder"),
                body: "/unrecognized with args".to_string(),
            }
        );
        assert_eq!(
            parse("/ask builder /fast"),
            ParsedInput::TargetedSlash {
                member: MemberId::new("builder"),
                body: "/fast".to_string(),
            }
        );
        assert_eq!(
            parse("/ask builder /attach"),
            ParsedInput::Attach {
                member: MemberId::new("builder"),
            }
        );
        for input in ["@all /attach", "/ask all /fast", "/all /fast"] {
            assert!(
                matches!(parse(input), ParsedInput::Invalid(message) if message.contains("one member")),
                "{input} must not broadcast a native-looking slash command"
            );
        }
    }

    #[test]
    fn attach_rejects_missing_or_extra_arguments() {
        assert!(matches!(parse("/attach"), ParsedInput::Help));
        assert!(matches!(
            parse("/attach builder extra"),
            ParsedInput::Invalid(_)
        ));
        assert!(matches!(
            parse("@builder /attach extra"),
            ParsedInput::Invalid(_)
        ));
    }

    #[test]
    fn plan_and_focus_commands() {
        assert_eq!(parse("/plan build a parser"), ParsedInput::Help);
        assert_eq!(
            parse("/focus reviewer"),
            ParsedInput::Surface(Surface::MemberLogs(MemberId::new("reviewer")))
        );
        assert_eq!(
            parse("/continue"),
            ParsedInput::Runtime(UiCommand::ContinueRun {
                run_id: None,
                note: None
            })
        );
        assert_eq!(
            parse("/continue run-12 fix verification"),
            ParsedInput::Runtime(UiCommand::ContinueRun {
                run_id: Some(RunId(12)),
                note: Some("fix verification".to_string())
            })
        );
        assert_eq!(parse("/cont unblock deployment"), ParsedInput::Help);
        assert_eq!(
            parse("/note run-12 waiting for product signoff"),
            ParsedInput::Runtime(UiCommand::NoteRun {
                run_id: Some(RunId(12)),
                note: "waiting for product signoff".to_string()
            })
        );
        assert_eq!(
            parse("/note checkpoint saved"),
            ParsedInput::Runtime(UiCommand::NoteRun {
                run_id: None,
                note: "checkpoint saved".to_string()
            })
        );
        assert_eq!(
            parse("/block run-12 missing credentials"),
            ParsedInput::Runtime(UiCommand::BlockRun {
                run_id: Some(RunId(12)),
                reason: "missing credentials".to_string()
            })
        );
        assert_eq!(
            parse("/step add write parser tests"),
            ParsedInput::Runtime(UiCommand::AddRunStep {
                run_id: None,
                owner: None,
                title: "write parser tests".to_string()
            })
        );
        assert_eq!(
            parse("/step add run-12 wire verification"),
            ParsedInput::Runtime(UiCommand::AddRunStep {
                run_id: Some(RunId(12)),
                owner: None,
                title: "wire verification".to_string()
            })
        );
        assert_eq!(
            parse("/step add run-12 @builder wire verification"),
            ParsedInput::Runtime(UiCommand::AddRunStep {
                run_id: Some(RunId(12)),
                owner: Some(MemberId::new("builder")),
                title: "wire verification".to_string()
            })
        );
        assert_eq!(
            parse("/step doing run-12 2 waiting on reviewer"),
            ParsedInput::Runtime(UiCommand::UpdateRunStep {
                run_id: Some(RunId(12)),
                step: 2,
                status: RunStepStatus::Doing,
                note: Some("waiting on reviewer".to_string())
            })
        );
        assert_eq!(
            parse("/step done 1"),
            ParsedInput::Runtime(UiCommand::UpdateRunStep {
                run_id: None,
                step: 1,
                status: RunStepStatus::Done,
                note: None
            })
        );
        assert_eq!(
            parse("/step rename run-12 2 document setup"),
            ParsedInput::Runtime(UiCommand::RenameRunStep {
                run_id: Some(RunId(12)),
                step: 2,
                title: "document setup".to_string()
            })
        );
        assert_eq!(
            parse("/step remove 3"),
            ParsedInput::Runtime(UiCommand::RemoveRunStep {
                run_id: None,
                step: 3
            })
        );
        assert_eq!(
            parse("/step assign run-12 3 reviewer"),
            ParsedInput::Runtime(UiCommand::AssignRunStep {
                run_id: Some(RunId(12)),
                step: 3,
                owner: Some(MemberId::new("reviewer"))
            })
        );
        assert_eq!(
            parse("/step unassign 3"),
            ParsedInput::Runtime(UiCommand::AssignRunStep {
                run_id: None,
                step: 3,
                owner: None
            })
        );
        assert_eq!(parse("/step add"), ParsedInput::Help);
        assert_eq!(parse("/step done 0"), ParsedInput::Help);
        assert_eq!(parse("/step done nope"), ParsedInput::Help);
        assert_eq!(parse("/step rename 2"), ParsedInput::Help);
        assert_eq!(parse("/step remove 0"), ParsedInput::Help);
        assert_eq!(parse("/step assign 2"), ParsedInput::Help);
        assert_eq!(parse("/note"), ParsedInput::Help);
        assert_eq!(parse("/block run-12"), ParsedInput::Help);
        assert_eq!(parse("/plan"), ParsedInput::Help);
        assert_eq!(parse("/focus"), ParsedInput::Help);
    }

    #[test]
    fn removed_mode_commands_do_not_bypass_mode_selection() {
        for command in [
            "/review fix it",
            "/plan goal",
            "/lead goal",
            "/roundtable topic",
            "/rt topic",
        ] {
            assert_eq!(parse(command), ParsedInput::Help);
        }
        assert_eq!(
            parse("/find needle"),
            ParsedInput::FindInChat("needle".to_string())
        );
        assert_eq!(parse("/find"), ParsedInput::FindInChat(String::new()));
    }

    #[test]
    fn verify_command_runs_default_or_explicit_check() {
        assert_eq!(
            parse("/verify"),
            ParsedInput::Runtime(UiCommand::VerifyRun {
                run_id: None,
                command: None
            })
        );
        assert_eq!(
            parse("/verify cargo test -q"),
            ParsedInput::Runtime(UiCommand::VerifyRun {
                run_id: None,
                command: Some("cargo test -q".to_string())
            })
        );
        assert_eq!(
            parse("/verify run-12 cargo test -q"),
            ParsedInput::Runtime(UiCommand::VerifyRun {
                run_id: Some(RunId(12)),
                command: Some("cargo test -q".to_string())
            })
        );
        assert_eq!(
            parse("/verify run-12"),
            ParsedInput::Runtime(UiCommand::VerifyRun {
                run_id: Some(RunId(12)),
                command: None
            })
        );
    }

    #[test]
    fn new_and_clear_both_start_a_fresh_session() {
        assert_eq!(parse("/new"), ParsedInput::Runtime(UiCommand::NewSession));
        assert_eq!(parse("/clear"), ParsedInput::Runtime(UiCommand::NewSession));
    }

    #[test]
    fn resume_opens_saved_chat_picker() {
        assert_eq!(
            parse("/resume"),
            ParsedInput::Runtime(UiCommand::RequestResume)
        );
        assert!(matches!(parse("/resume 3"), ParsedInput::Invalid(_)));
    }

    #[test]
    fn mode_command_selects_normal_and_collaboration_modes() {
        for (text, mode) in [
            ("/mode normal", TerminalMode::Normal),
            ("/mode review", TerminalMode::Review),
            ("/mode plan", TerminalMode::Plan),
            ("/mode brainstorm", TerminalMode::Brainstorm),
            ("/mode team", TerminalMode::Team),
        ] {
            assert_eq!(
                parse(text),
                ParsedInput::Runtime(UiCommand::SetMode { mode })
            );
        }
        assert_eq!(parse("/mode"), ParsedInput::Surface(Surface::Mode));
        assert_eq!(parse("/mode review fix parser"), ParsedInput::Help);
    }

    #[test]
    fn removed_skills_command_falls_back_to_help() {
        assert_eq!(parse("/skills"), ParsedInput::Help);
        assert_eq!(parse("/skill"), ParsedInput::Help);
    }

    #[test]
    fn removed_abort_command_falls_back_to_help() {
        assert_eq!(parse("/abort"), ParsedInput::Help);
        assert_eq!(parse("/abort extra"), ParsedInput::Help);
    }

    #[test]
    fn import_and_export_commands_parse_correctly() {
        assert_eq!(
            parse("/import sess-1234"),
            ParsedInput::Runtime(UiCommand::ImportSession {
                member: None,
                session_id: "sess-1234".to_string(),
            })
        );
        assert_eq!(
            parse("/import builder sess-1234"),
            ParsedInput::Runtime(UiCommand::ImportSession {
                member: Some(MemberId::new("builder")),
                session_id: "sess-1234".to_string(),
            })
        );
        assert_eq!(
            parse("@builder /import sess-1234"),
            ParsedInput::Runtime(UiCommand::ImportSession {
                member: Some(MemberId::new("builder")),
                session_id: "sess-1234".to_string(),
            })
        );
        assert_eq!(parse("/import"), ParsedInput::Help);

        assert_eq!(
            parse("/export"),
            ParsedInput::Runtime(UiCommand::ExportSession { format: None })
        );
        assert_eq!(
            parse("/export claude"),
            ParsedInput::Runtime(UiCommand::ExportSession {
                format: Some("claude".to_string())
            })
        );
    }

    /// Table-driven contract check: every catalog command is documented and
    /// parses to the documented arity behaviour. This is the shared half of the
    /// TUI/Desktop parser-consistency guarantee; the TUI wrapper and the
    /// desktop bridge each add their own mapping tests.
    #[test]
    fn catalog_matches_documented_commands_and_arity() {
        for spec in COMMAND_CATALOG {
            let heading = format!("### `/{}`", spec.name);
            assert!(
                include_str!("../../docs/commands.md").contains(&heading),
                "command reference is missing {heading}"
            );
            assert!(
                include_str!("../../docs/commands.zh-CN.md").contains(&heading),
                "Chinese command reference is missing {heading}"
            );
            // No-argument commands reject trailing text; argument commands
            // accept at least the empty-arg form without panicking.
            if spec.takes_argument {
                assert!(matches!(
                    parse(&format!("/{}", spec.name)),
                    ParsedInput::Help
                        | ParsedInput::Runtime(_)
                        | ParsedInput::Surface(_)
                        | ParsedInput::FindInChat(_)
                        | ParsedInput::Attach { .. }
                        | ParsedInput::Exit
                ));
            } else {
                assert!(matches!(
                    parse(&format!("/{} extra", spec.name)),
                    ParsedInput::Invalid(_)
                ));
            }
        }
    }

    #[test]
    fn catalog_covers_every_parseable_top_level_command() {
        // The parser must never accept a command the catalog does not list.
        let catalog: Vec<&str> = COMMAND_CATALOG.iter().map(|spec| spec.name).collect();
        for command in [
            "ask", "all", "attach", "approve", "block", "continue", "diff", "exit", "export",
            "find", "focus", "import", "logs", "mode", "new", "note", "reject", "resume", "retry",
            "runs", "step", "team", "verify",
        ] {
            assert!(
                catalog.contains(&command),
                "parser accepts /{command} but the catalog does not list it"
            );
            assert!(matches!(
                parse(&format!("/{command}")),
                ParsedInput::Help
                    | ParsedInput::Runtime(_)
                    | ParsedInput::Surface(_)
                    | ParsedInput::FindInChat(_)
                    | ParsedInput::Attach { .. }
                    | ParsedInput::Exit
                    | ParsedInput::ApproveFirst(_)
            ));
        }
        assert_eq!(
            parse("/clear"),
            parse("/new"),
            "the /clear alias must behave exactly like /new"
        );
    }
}
