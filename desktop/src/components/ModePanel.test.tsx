import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { demoSnapshot } from "../bridge/mock";
import { createTranslator } from "../i18n";
import { ModePanel } from "./ModePanel";

describe("ModePanel", () => {
  it("switches modes from the bare /mode panel", async () => {
    const dispatch = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(
      <ModePanel
        mode="review"
        defaults={demoSnapshot.team!.modes}
        overrides={{}}
        members={demoSnapshot.members}
        busy={false}
        t={createTranslator("en-US")}
        onClose={() => undefined}
        dispatch={dispatch}
      />,
    );

    await user.click(screen.getByRole("tab", { name: "Team" }));
    expect(dispatch).toHaveBeenCalledWith({ type: "set_mode", mode: "team" });
  });

  it("shows the v1.0.4 review field instead of removed verification controls", () => {
    render(
      <ModePanel
        mode="review"
        defaults={demoSnapshot.team!.modes}
        overrides={{}}
        members={demoSnapshot.members}
        busy={false}
        t={createTranslator("en-US")}
        onClose={() => undefined}
        dispatch={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(screen.getByDisplayValue(/accessibility before approving/i)).toBeInTheDocument();
    expect(screen.queryByText("Auto verify")).not.toBeInTheDocument();
    expect(screen.queryByText("Verify command")).not.toBeInTheDocument();
  });
});
