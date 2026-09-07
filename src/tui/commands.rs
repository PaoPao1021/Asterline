//! TUI submission vocabulary over the shared composer contract.
//!
//! Parsing, the command catalog, and completion live in [`crate::contract`]
//! so the desktop GUI accepts exactly the same commands; this module maps the
//! UI-agnostic [`crate::contract::ParsedInput`] onto the TUI's [`Submission`].

use crate::contract::{self, ParsedInput, Surface};
use crate::domain::event::{ApprovalDecision, MessageTarget, UiCommand};
use crate::domain::team::MemberId;
use crate::tui::drawers::Drawer;

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Submission {
    Exit,
    Attach { member: MemberId },
    TargetedSlash { member: MemberId, body: String },
    Runtime(UiCommand),
    Drawer(Drawer),
    ApproveFirst(ApprovalDecision),
    FindInChat(String),
    Help,
    Invalid(String),
    NeedsTarget,
    Empty,
}

impl From<ParsedInput> for Submission {
    fn from(input: ParsedInput) -> Self {
        match input {
            ParsedInput::Exit => Self::Exit,
            ParsedInput::Attach { member } => Self::Attach { member },
            ParsedInput::TargetedSlash { member, body } => Self::TargetedSlash { member, body },
            ParsedInput::Runtime(command) => Self::Runtime(command),
            ParsedInput::Surface(surface) => Self::Drawer(drawer_from_surface(surface)),
            ParsedInput::ApproveFirst(decision) => Self::ApproveFirst(decision),
            ParsedInput::FindInChat(query) => Self::FindInChat(query),
            ParsedInput::Help => Self::Help,
            ParsedInput::Invalid(message) => Self::Invalid(message),
            ParsedInput::NeedsTarget => Self::NeedsTarget,
            ParsedInput::Empty => Self::Empty,
        }
    }
}

fn drawer_from_surface(surface: Surface) -> Drawer {
    match surface {
        Surface::Team => Drawer::Team,
        Surface::Runs => Drawer::Runs,
        Surface::Logs => Drawer::Logs,
        Surface::Diff => Drawer::Diff,
        Surface::Mode => Drawer::Mode,
        Surface::MemberLogs(member) => Drawer::MemberLogs(member),
    }
}

pub fn parse(input: &str) -> Submission {
    contract::parse(input).into()
}

pub fn parse_target_only(input: &str) -> Option<MessageTarget> {
    contract::parse_target_only(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::{RunId, RunStepStatus};
    use crate::domain::mode::TerminalMode;

    #[test]
    fn shared_contract_maps_onto_tui_submissions() {
        assert_eq!(parse("build the parser"), Submission::NeedsTarget);
        assert_eq!(parse("   "), Submission::Empty);
        assert_eq!(parse("/wat"), Submission::Help);
        assert_eq!(parse("/logs"), Submission::Drawer(Drawer::Logs));
        assert_eq!(parse("/runs"), Submission::Drawer(Drawer::Runs));
        assert_eq!(parse("/team"), Submission::Drawer(Drawer::Team));
        assert_eq!(parse("/diff"), Submission::Drawer(Drawer::Diff));
        assert_eq!(parse("/mode"), Submission::Drawer(Drawer::Mode));
        assert_eq!(
            parse("/focus reviewer"),
            Submission::Drawer(Drawer::MemberLogs(MemberId::new("reviewer")))
        );
        assert_eq!(parse("/exit"), Submission::Exit);
        assert_eq!(parse("/retry"), Submission::Runtime(UiCommand::Retry));
        assert_eq!(
            parse("/approve"),
            Submission::ApproveFirst(ApprovalDecision::Approve)
        );
        assert_eq!(
            parse("/ask builder implement it"),
            Submission::Runtime(UiCommand::UserMessage {
                target: MessageTarget::Member(MemberId::new("builder")),
                body: "@builder implement it".to_string(),
            })
        );
        assert_eq!(
            parse("/mode team"),
            Submission::Runtime(UiCommand::SetMode {
                mode: TerminalMode::Team
            })
        );
        assert_eq!(
            parse("/continue run-12 unblock delivery"),
            Submission::Runtime(UiCommand::ContinueRun {
                run_id: Some(RunId(12)),
                note: Some("unblock delivery".to_string())
            })
        );
        assert_eq!(
            parse("/step done 1"),
            Submission::Runtime(UiCommand::UpdateRunStep {
                run_id: None,
                step: 1,
                status: RunStepStatus::Done,
                note: None
            })
        );
        assert_eq!(
            parse("/find needle"),
            Submission::FindInChat("needle".to_string())
        );
        assert_eq!(
            parse("@builder /unrecognized with args"),
            Submission::TargetedSlash {
                member: MemberId::new("builder"),
                body: "/unrecognized with args".to_string(),
            }
        );
    }

    #[test]
    fn tui_and_contract_accept_identical_commands() {
        for (text, expected) in [
            ("/new", Submission::Runtime(UiCommand::NewSession)),
            ("/clear", Submission::Runtime(UiCommand::NewSession)),
            ("/resume", Submission::Runtime(UiCommand::RequestResume)),
            (
                "/reject",
                Submission::ApproveFirst(ApprovalDecision::Reject),
            ),
            ("/help", Submission::Help),
        ] {
            assert_eq!(parse(text), expected);
            assert_eq!(Submission::from(contract::parse(text)), expected);
        }
        assert_eq!(
            parse_target_only("@all"),
            contract::parse_target_only("@all")
        );
    }

    #[test]
    fn image_only_sends_keep_an_explicit_target() {
        assert_eq!(
            parse_target_only("@builder"),
            Some(MessageTarget::Member(MemberId::new("builder")))
        );
        assert_eq!(parse_target_only("/all"), Some(MessageTarget::All));
        assert_eq!(parse_target_only("@builder look"), None);
    }
}
