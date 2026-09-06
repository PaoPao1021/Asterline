//! Desktop surface over the shared composer contract.
//!
//! The WebView no longer parses composer text itself: every submission goes
//! through [`parse_composer_text`], which consumes the same `crate::contract`
//! parser as the TUI, resolves targeted `@member /skill` invocations against
//! discovered skills, and produces either a runtime command or a GUI action.
//! The command catalog and completion feed palettes, `/help`, and suggestions.

use std::collections::HashMap;

use serde::Serialize;

use asterline::contract::{self, completion};
use asterline::domain::event::ApprovalDecision;
use asterline::domain::event::UiCommand;
use asterline::domain::team::BackendKind;
use asterline::tui::skills::SkillInfo;

use crate::bridge::{
    ApprovalChoiceV2, DesktopCommandV2, MemberSummaryV2, MessageTargetV2, TerminalModeV2,
};

/// A panel the desktop opens in response to a parsed submission.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComposerSurfaceV2 {
    Team,
    Runs,
    Logs,
    Diff,
    Mode,
    MemberLogs,
}

/// What the composer submission should do, in desktop vocabulary.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)] // Runtime commands stay unboxed at the UI boundary.
pub enum ComposerActionV2 {
    Empty,
    Exit,
    Attach {
        member: String,
    },
    Command {
        command: DesktopCommandV2,
    },
    Surface {
        surface: ComposerSurfaceV2,
        member: Option<String>,
    },
    ApproveFirst {
        decision: ApprovalChoiceV2,
    },
    Find {
        query: String,
    },
    Help,
    Invalid {
        message: String,
    },
    NeedsTarget,
}

/// Parse composer text exactly like the TUI does, then map the UI-agnostic
/// result onto desktop commands and surfaces. `@member /skill` bodies are
/// resolved here so undiscovered skills never reach a backend.
pub fn parse_composer_text(
    text: &str,
    members: &[MemberSummaryV2],
    skills: &[SkillInfo],
) -> ComposerActionV2 {
    match contract::parse(text) {
        // `@member` / `@all` / `/ask member` with no body is a target-only
        // submission (the TUI's `parse_target_only`): it sends nothing on its
        // own but carries staged images to the explicit target. The WebView
        // treats an empty body without attachments as a no-op.
        contract::ParsedInput::Empty => match contract::parse_target_only(text) {
            Some(asterline::domain::event::MessageTarget::All) => ComposerActionV2::Command {
                command: DesktopCommandV2::UserMessage {
                    target: MessageTargetV2::All,
                    body: String::new(),
                    attachments: Vec::new(),
                },
            },
            Some(asterline::domain::event::MessageTarget::Member(member)) => {
                ComposerActionV2::Command {
                    command: DesktopCommandV2::UserMessage {
                        target: MessageTargetV2::Member {
                            member: member.to_string(),
                        },
                        body: String::new(),
                        attachments: Vec::new(),
                    },
                }
            }
            _ => ComposerActionV2::Empty,
        },
        contract::ParsedInput::Exit => ComposerActionV2::Exit,
        contract::ParsedInput::NeedsTarget => ComposerActionV2::NeedsTarget,
        contract::ParsedInput::Help => ComposerActionV2::Help,
        contract::ParsedInput::Invalid(message) => ComposerActionV2::Invalid { message },
        contract::ParsedInput::FindInChat(query) => ComposerActionV2::Find { query },
        contract::ParsedInput::ApproveFirst(decision) => ComposerActionV2::ApproveFirst {
            decision: match decision {
                ApprovalDecision::Approve => ApprovalChoiceV2::Approve,
                ApprovalDecision::Reject => ApprovalChoiceV2::Reject,
            },
        },
        contract::ParsedInput::Attach { member } => ComposerActionV2::Attach {
            member: member.to_string(),
        },
        contract::ParsedInput::TargetedSlash { member, body } => {
            resolve_targeted_slash(member.as_str(), &body, members, skills)
        }
        contract::ParsedInput::Runtime(command) => match ui_command_to_desktop(command) {
            Some(command) => ComposerActionV2::Command { command },
            None => ComposerActionV2::Invalid {
                message: "this command is not available in the desktop composer".to_string(),
            },
        },
        contract::ParsedInput::Surface(surface) => match surface {
            contract::Surface::Team => ComposerActionV2::Surface {
                surface: ComposerSurfaceV2::Team,
                member: None,
            },
            contract::Surface::Runs => ComposerActionV2::Surface {
                surface: ComposerSurfaceV2::Runs,
                member: None,
            },
            contract::Surface::Logs => ComposerActionV2::Surface {
                surface: ComposerSurfaceV2::Logs,
                member: None,
            },
            contract::Surface::Diff => ComposerActionV2::Surface {
                surface: ComposerSurfaceV2::Diff,
                member: None,
            },
            contract::Surface::Mode => ComposerActionV2::Surface {
                surface: ComposerSurfaceV2::Mode,
                member: None,
            },
            contract::Surface::MemberLogs(member) => ComposerActionV2::Surface {
                surface: ComposerSurfaceV2::MemberLogs,
                member: Some(member.to_string()),
            },
        },
    }
}

/// Validate a `@member /skill` body against the target backend's discovered
/// prompt-invocable skills, mirroring the TUI's `targeted_skill_command`.
fn resolve_targeted_slash(
    member: &str,
    body: &str,
    members: &[MemberSummaryV2],
    skills: &[SkillInfo],
) -> ComposerActionV2 {
    let Some(summary) = members
        .iter()
        .find(|candidate| {
            candidate.id.eq_ignore_ascii_case(member)
                || candidate.display_name.eq_ignore_ascii_case(member)
        })
        .cloned()
    else {
        return ComposerActionV2::Invalid {
            message: format!("{member} is not a team member"),
        };
    };
    let backend = domain_backend(summary.backend);
    let token = body.split_whitespace().next().unwrap_or_default();
    let invocation = if backend == BackendKind::Codex && token.starts_with('/') {
        format!("${}", token.trim_start_matches('/'))
    } else {
        token.to_string()
    };
    if !skills
        .iter()
        .any(|skill| skill.backend == backend && skill.invocation == invocation)
    {
        return ComposerActionV2::Invalid {
            message: format!(
                "{body} is not a discovered prompt-invocable skill for {member}; use /attach <member> for that backend's native CLI"
            ),
        };
    }
    // Codex skills are invoked with a `$name` prefix; normalize the body the
    // same way the TUI does before submitting.
    let body = if backend == BackendKind::Codex {
        match body.strip_prefix('/') {
            Some(rest) => format!("${rest}"),
            None => body.to_string(),
        }
    } else {
        body.to_string()
    };
    ComposerActionV2::Command {
        command: DesktopCommandV2::UserMessage {
            target: MessageTargetV2::Member {
                member: summary.id.clone(),
            },
            body: format!("@{} {}", summary.id, body),
            attachments: Vec::new(),
        },
    }
}

fn domain_backend(backend: crate::bridge::BackendKindV2) -> BackendKind {
    match backend {
        crate::bridge::BackendKindV2::Codex => BackendKind::Codex,
        crate::bridge::BackendKindV2::Claude => BackendKind::Claude,
        crate::bridge::BackendKindV2::Grok => BackendKind::Grok,
        crate::bridge::BackendKindV2::Agy => BackendKind::Agy,
    }
}

/// Translate the parser's runtime command into the desktop command vocabulary.
fn ui_command_to_desktop(command: UiCommand) -> Option<DesktopCommandV2> {
    Some(match command {
        UiCommand::SetMode { mode } => DesktopCommandV2::SetMode {
            mode: terminal_mode(mode.as_str()),
        },
        // The parser keeps the `@member` mention inside the body (that is what
        // the runtime persists), so the desktop body passes through verbatim.
        UiCommand::UserMessage { target, body } => {
            let target = match target {
                asterline::domain::event::MessageTarget::All => MessageTargetV2::All,
                asterline::domain::event::MessageTarget::Member(id) => MessageTargetV2::Member {
                    member: id.to_string(),
                },
                _ => return None,
            };
            DesktopCommandV2::UserMessage {
                target,
                body,
                attachments: Vec::new(),
            }
        }
        UiCommand::NewSession => DesktopCommandV2::NewSession,
        UiCommand::RequestResume => DesktopCommandV2::RequestResume,
        UiCommand::Retry => DesktopCommandV2::Retry,
        UiCommand::ContinueRun { run_id, note } => DesktopCommandV2::ContinueRun {
            run_id: run_id.map(|id| id.0),
            note,
        },
        UiCommand::NoteRun { run_id, note } => DesktopCommandV2::NoteRun {
            run_id: run_id.map(|id| id.0),
            note,
        },
        UiCommand::BlockRun { run_id, reason } => DesktopCommandV2::BlockRun {
            run_id: run_id.map(|id| id.0),
            reason,
        },
        UiCommand::VerifyRun { run_id, command } => DesktopCommandV2::VerifyRun {
            run_id: run_id.map(|id| id.0),
            command,
        },
        UiCommand::AddRunStep {
            run_id,
            owner,
            title,
        } => DesktopCommandV2::AddRunStep {
            run_id: run_id.map(|id| id.0),
            owner: owner.map(|id| id.to_string()),
            title,
        },
        UiCommand::UpdateRunStep {
            run_id,
            step,
            status,
            note,
        } => DesktopCommandV2::UpdateRunStep {
            run_id: run_id.map(|id| id.0),
            step,
            status: run_step_status(status.as_str()),
            note,
        },
        UiCommand::RenameRunStep {
            run_id,
            step,
            title,
        } => DesktopCommandV2::RenameRunStep {
            run_id: run_id.map(|id| id.0),
            step,
            title,
        },
        UiCommand::RemoveRunStep { run_id, step } => DesktopCommandV2::RemoveRunStep {
            run_id: run_id.map(|id| id.0),
            step,
        },
        UiCommand::AssignRunStep {
            run_id,
            step,
            owner,
        } => DesktopCommandV2::AssignRunStep {
            run_id: run_id.map(|id| id.0),
            step,
            owner: owner.map(|id| id.to_string()),
        },
        UiCommand::ImportSession { member, session_id } => DesktopCommandV2::ImportSession {
            member: member.map(|id| id.to_string()),
            session_id,
        },
        UiCommand::ExportSession { format } => DesktopCommandV2::ExportSession { format },
        _ => return None,
    })
}

fn run_step_status(value: &str) -> crate::bridge::RunStepStatusV2 {
    match value {
        "doing" => crate::bridge::RunStepStatusV2::Doing,
        "done" => crate::bridge::RunStepStatusV2::Done,
        "blocked" => crate::bridge::RunStepStatusV2::Blocked,
        _ => crate::bridge::RunStepStatusV2::Todo,
    }
}

fn terminal_mode(value: &str) -> TerminalModeV2 {
    match value {
        "review" => TerminalModeV2::Review,
        "plan" => TerminalModeV2::Plan,
        "brainstorm" => TerminalModeV2::Brainstorm,
        "team" => TerminalModeV2::Team,
        _ => TerminalModeV2::Normal,
    }
}

// --- catalog & completion -------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CommandSpecV2 {
    pub name: &'static str,
    pub hint: &'static str,
    pub takes_argument: bool,
}

/// The shared command catalog, for the GUI command palette and `/help`.
pub fn command_catalog() -> Vec<CommandSpecV2> {
    contract::COMMAND_CATALOG
        .iter()
        .map(|spec| CommandSpecV2 {
            name: spec.name,
            hint: spec.hint,
            takes_argument: spec.takes_argument,
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CompletionItemV2 {
    pub label: String,
    pub insert: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CompletionV2 {
    pub title: String,
    pub token_start: usize,
    pub items: Vec<CompletionItemV2>,
}

/// Shared completion for the composer: commands, modes, members, and — after
/// an explicit `@member /` — that backend's discovered skills.
pub fn complete_composer(
    head: &str,
    members: &[MemberSummaryV2],
    skills: &[SkillInfo],
) -> Option<CompletionV2> {
    let member_names: Vec<String> = members.iter().map(|member| member.id.clone()).collect();
    let member_backends: HashMap<String, BackendKind> = members
        .iter()
        .map(|member| (member.id.clone(), domain_backend(member.backend)))
        .collect();
    let agent_skills: Vec<completion::AgentSkill> = skills
        .iter()
        .map(|skill| completion::AgentSkill {
            name: skill.name.clone(),
            description: skill.description.clone(),
            backend: skill.backend,
            invocation: skill.invocation.clone(),
        })
        .collect();
    completion::compute_with_agent_skills(head, &member_names, &agent_skills, &member_backends).map(
        |found| CompletionV2 {
            title: found.title.to_string(),
            token_start: found.token_start,
            items: found
                .items
                .into_iter()
                .map(|item| CompletionItemV2 {
                    label: item.label,
                    insert: item.insert,
                })
                .collect(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::{BackendKindV2, MemberSummaryV2};
    use std::path::{Path, PathBuf};

    fn member(id: &str, backend: BackendKindV2) -> MemberSummaryV2 {
        MemberSummaryV2 {
            id: id.to_string(),
            display_name: id.to_string(),
            backend,
            role: "general".to_string(),
            status: crate::bridge::MemberStatusV2::Idle,
            session: None,
            cwd: "/workspace".to_string(),
            model: None,
            effort: None,
            sandbox: crate::bridge::SandboxPolicyV2::ReadOnly,
            permission_mode: None,
            session_policy: crate::bridge::SessionPolicyV2::Resume,
        }
    }

    fn skill(name: &str, backend: BackendKind) -> SkillInfo {
        SkillInfo {
            name: name.to_string(),
            description: format!("{name} description"),
            path: std::path::PathBuf::from("/skills"),
            backend,
            invocation: format!("${name}"),
        }
    }

    #[test]
    fn target_only_submissions_carry_images_to_an_explicit_target() {
        // Like the TUI's `parse_target_only`: a bare `@member` sends nothing
        // by itself, but staged images must reach the named target.
        let members = vec![member("builder", BackendKindV2::Codex)];
        let action = parse_composer_text("@builder", &members, &[]);
        let ComposerActionV2::Command { command } = action else {
            panic!("expected command");
        };
        let DesktopCommandV2::UserMessage { target, body, .. } = command else {
            panic!("expected user message");
        };
        assert_eq!(
            target,
            MessageTargetV2::Member {
                member: "builder".to_string()
            }
        );
        assert_eq!(body, "");

        let action = parse_composer_text("@all", &members, &[]);
        let ComposerActionV2::Command { command } = action else {
            panic!("expected command");
        };
        let DesktopCommandV2::UserMessage { target, .. } = command else {
            panic!("expected user message");
        };
        assert_eq!(target, MessageTargetV2::All);

        // The boundary rejects an empty body without attachments, so the
        // draft is kept when no image is staged.
        let resolve = |token: &str| -> Result<PathBuf, String> {
            Err(format!("unknown attachment: {token}"))
        };
        let error = crate::session_adapter::command_to_runtime(
            DesktopCommandV2::UserMessage {
                target: MessageTargetV2::Member {
                    member: "builder".to_string(),
                },
                body: String::new(),
                attachments: Vec::new(),
            },
            Path::new("/workspace"),
            &resolve,
        )
        .unwrap_err();
        assert!(error.contains("message cannot be empty"));
    }

    #[test]
    fn plain_text_without_target_is_rejected() {
        let members = vec![member("builder", BackendKindV2::Codex)];
        assert_eq!(
            parse_composer_text("hello there", &members, &[]),
            ComposerActionV2::NeedsTarget
        );
        assert_eq!(
            parse_composer_text("  ", &members, &[]),
            ComposerActionV2::Empty
        );
    }

    #[test]
    fn mention_becomes_a_structured_user_message() {
        let members = vec![member("builder", BackendKindV2::Codex)];
        let action = parse_composer_text("@builder please look", &members, &[]);
        let ComposerActionV2::Command { command } = action else {
            panic!("expected command, got {action:?}");
        };
        let DesktopCommandV2::UserMessage {
            target,
            body,
            attachments,
        } = command
        else {
            panic!("expected user message");
        };
        assert_eq!(
            target,
            MessageTargetV2::Member {
                member: "builder".to_string()
            }
        );
        assert_eq!(body, "@builder please look");
        assert!(attachments.is_empty());
    }

    #[test]
    fn slash_commands_map_to_desktop_commands() {
        let members = vec![member("builder", BackendKindV2::Codex)];
        for (text, expected) in [
            ("/new", DesktopCommandV2::NewSession),
            ("/clear", DesktopCommandV2::NewSession),
            ("/resume", DesktopCommandV2::RequestResume),
            ("/retry", DesktopCommandV2::Retry),
            (
                "/mode plan",
                DesktopCommandV2::SetMode {
                    mode: TerminalModeV2::Plan,
                },
            ),
            (
                "/verify run-7 cargo test",
                DesktopCommandV2::VerifyRun {
                    run_id: Some(7),
                    command: Some("cargo test".to_string()),
                },
            ),
            (
                "/import builder sess-9",
                DesktopCommandV2::ImportSession {
                    member: Some("builder".to_string()),
                    session_id: "sess-9".to_string(),
                },
            ),
            (
                "/export claude",
                DesktopCommandV2::ExportSession {
                    format: Some("claude".to_string()),
                },
            ),
        ] {
            assert_eq!(
                parse_composer_text(text, &members, &[]),
                ComposerActionV2::Command { command: expected },
                "{text}"
            );
        }
    }

    #[test]
    fn surfaces_map_to_gui_panels() {
        let members = vec![member("builder", BackendKindV2::Codex)];
        assert_eq!(
            parse_composer_text("/logs", &members, &[]),
            ComposerActionV2::Surface {
                surface: ComposerSurfaceV2::Logs,
                member: None,
            }
        );
        assert_eq!(
            parse_composer_text("/mode", &members, &[]),
            ComposerActionV2::Surface {
                surface: ComposerSurfaceV2::Mode,
                member: None,
            }
        );
        assert_eq!(
            parse_composer_text("/focus reviewer", &members, &[]),
            ComposerActionV2::Surface {
                surface: ComposerSurfaceV2::MemberLogs,
                member: Some("reviewer".to_string()),
            }
        );
        assert_eq!(
            parse_composer_text("/exit", &members, &[]),
            ComposerActionV2::Exit
        );
    }

    #[test]
    fn targeted_skill_must_be_discovered_for_that_backend() {
        let members = vec![
            member("codex", BackendKindV2::Codex),
            member("claude", BackendKindV2::Claude),
        ];
        let skills = vec![skill("review", BackendKind::Codex)];

        let action = parse_composer_text("@codex /review", &members, &skills);
        let ComposerActionV2::Command { command } = action else {
            panic!("expected command");
        };
        let DesktopCommandV2::UserMessage { body, target, .. } = command else {
            panic!("expected user message");
        };
        assert_eq!(body, "@codex $review");
        assert_eq!(
            target,
            MessageTargetV2::Member {
                member: "codex".to_string()
            }
        );

        // Claude has no $review skill: the invocation must be refused.
        assert!(matches!(
            parse_composer_text("@claude /review", &members, &skills),
            ComposerActionV2::Invalid { .. }
        ));
        // Unknown members are refused too.
        assert!(matches!(
            parse_composer_text("@stranger /review", &members, &skills),
            ComposerActionV2::Invalid { .. }
        ));
        // Native controls never pass as skills.
        assert!(matches!(
            parse_composer_text("@codex /model gpt-9", &members, &skills),
            ComposerActionV2::Invalid { .. }
        ));
    }

    #[test]
    fn catalog_lists_every_shared_command() {
        let catalog = command_catalog();
        assert!(catalog.len() >= 24);
        assert!(catalog.iter().any(|spec| spec.name == "exit"));
        assert!(catalog.iter().any(|spec| spec.name == "export"));
        assert!(
            catalog
                .iter()
                .all(|spec| !spec.name.is_empty() && !spec.hint.is_empty())
        );
    }

    #[test]
    fn completion_covers_commands_members_and_skills() {
        let members = vec![member("builder", BackendKindV2::Codex)];
        let skills = vec![skill("review", BackendKind::Codex)];

        let commands = complete_composer("/re", &members, &skills).expect("commands");
        assert_eq!(commands.title, "commands");
        assert!(commands.items.iter().any(|item| item.insert == "/reject "));

        let mentions = complete_composer("@bu", &members, &skills).expect("members");
        assert!(mentions.items.iter().any(|item| item.insert == "@builder "));

        let targeted = complete_composer("@builder /re", &members, &skills).expect("skills");
        assert!(targeted.items.iter().any(|item| item.insert == "$review "));

        assert!(complete_composer("plain", &members, &skills).is_none());
    }
}
