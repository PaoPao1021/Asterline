import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { demoSnapshot } from "../bridge/mock";
import { createTranslator } from "../i18n";
import { SettingsModal, patchMember, replaceMemberReference } from "./SettingsModal";

describe("SettingsModal", () => {
  it("patchMember preserves bridge fields that the edited control does not own", () => {
    const source = demoSnapshot.team!.members[0];
    const next = patchMember(source, { display_name: "Lead Builder" });
    expect(next.system_prompt).toBe(source.system_prompt);
    expect(next.session_id).toBe(source.session_id);
    expect(next.allowed_tools).toEqual(source.allowed_tools);
  });

  it("preserves hidden member fields when saving a visible edit", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(<SettingsModal settings={structuredClone(demoSnapshot.team!)} busy={false} t={createTranslator("en-US")} onClose={() => undefined} onSave={onSave} />);

    await user.click(screen.getByRole("button", { name: "Members" }));
    const displayName = screen.getByRole("textbox", { name: "Display name" });
    await user.clear(displayName);
    await user.type(displayName, "Lead Builder");
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    expect(onSave).toHaveBeenCalledTimes(1);
    const saved = onSave.mock.calls[0][0] as typeof demoSnapshot.team;
    expect(saved!.members[0]).toMatchObject({
      display_name: "Lead Builder",
      system_prompt: demoSnapshot.team!.members[0].system_prompt,
      session_id: demoSnapshot.team!.members[0].session_id,
      allowed_tools: demoSnapshot.team!.members[0].allowed_tools,
    });
  });

  it("cascades renamed and removed member references", () => {
    const team = structuredClone(demoSnapshot.team!);
    const renamed = replaceMemberReference(team, "builder", "lead");
    expect(renamed.default_target).toEqual({ type: "member", member: "lead" });
    expect(renamed.modes.review?.builder).toBe("lead");
    expect(renamed.modes.plan?.leader).toBe("lead");
    expect(renamed.modes.team?.coordinator).toBe("lead");
    expect(renamed.modes.brainstorm?.participants).toContain("lead");

    const removed = replaceMemberReference(team, "builder", null);
    expect(removed.default_target).toBeNull();
    expect(removed.modes.review?.builder).toBeNull();
    expect(removed.modes.brainstorm?.participants).not.toContain("builder");
  });

  it("edits native manual approvals without exposing removed keyword gates", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(<SettingsModal settings={structuredClone(demoSnapshot.team!)} busy={false} t={createTranslator("en-US")} onClose={() => undefined} onSave={onSave} />);

    await user.click(screen.getByRole("button", { name: "Approvals" }));
    const manual = screen.getByRole("checkbox", { name: /Manual Codex tool approvals/ });
    expect(manual).toBeChecked();
    expect(screen.queryByText("Gate categories")).not.toBeInTheDocument();
    await user.click(manual);
    await user.click(screen.getByRole("button", { name: "Save changes" }));
    expect(onSave.mock.calls[0][0].approvals.manual).toBe(false);
  });

  it("rejects unsupported effort and brainstorm limits before submission", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn().mockResolvedValue(undefined);
    const invalid = structuredClone(demoSnapshot.team!);
    invalid.members[0].backend = "agy";
    invalid.members[0].effort = "max";
    invalid.modes.brainstorm = {
      ...invalid.modes.brainstorm,
      generation_rounds: 1,
      ideas_per_round: 2,
    };
    render(<SettingsModal settings={invalid} busy={false} t={createTranslator("en-US")} onClose={() => undefined} onSave={onSave} />);

    expect(screen.getByRole("button", { name: "Save changes" })).toBeDisabled();
    expect(screen.getByText("Add at least one member and give every member a unique handle and name.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Save changes" }));
    expect(onSave).not.toHaveBeenCalled();
  });
});
