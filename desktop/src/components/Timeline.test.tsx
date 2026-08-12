import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { demoSnapshot } from "../bridge/mock";
import { createTranslator } from "../i18n";
import { Timeline } from "./Timeline";

describe("Timeline paused route", () => {
  it("lets the user continue or drop the queued route", async () => {
    const user = userEvent.setup();
    const resolve = vi.fn().mockResolvedValue(undefined);
    render(<Timeline
      items={[{ id: "paused-1", kind: "route_paused", member: "builder", to: ["reviewer"], text: "relay limit reached", detail: "1" }]}
      members={demoSnapshot.members}
      locale="en-US"
      t={createTranslator("en-US")}
      onResolvePausedRoute={resolve}
    />);
    expect(screen.getByText("Automatic relay paused")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Continue route" }));
    expect(resolve).toHaveBeenCalledWith("paused-1", true);
  });

  it("only lets the user resolve the runtime queue head", () => {
    render(<Timeline
      items={[
        { id: "paused-1", kind: "route_paused", member: "builder", to: ["reviewer"], text: "first", detail: "2" },
        { id: "paused-2", kind: "route_paused", member: "reviewer", to: ["builder"], text: "second", detail: "1" },
      ]}
      members={demoSnapshot.members}
      locale="en-US"
      t={createTranslator("en-US")}
      onResolvePausedRoute={vi.fn().mockResolvedValue(undefined)}
    />);

    const continueButtons = screen.getAllByRole("button", { name: "Continue route" });
    expect(continueButtons[0]).toBeEnabled();
    expect(continueButtons[1]).toBeDisabled();
  });
});
