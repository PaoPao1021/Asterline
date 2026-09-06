import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { setDesktopClientForTests } from "../bridge/client";
import { createMockDesktopClient, demoSnapshot } from "../bridge/mock";
import type { StagedAttachment, TerminalMode } from "../bridge/types";
import { Composer } from "./Composer";

const t = (key: string, values?: Record<string, string | number>) =>
  ({ composerPlaceholder: "Message the team", target: "Target" } as Record<string, string>)[key] ?? key;

function setup() {
  const client = createMockDesktopClient();
  setDesktopClientForTests(client);
  const onSubmit = vi.fn(async (_text: string, _attachments: StagedAttachment[]) => true);
  const utils = render(
    <Composer
      members={demoSnapshot.members}
      mode={"normal" as TerminalMode}
      target="default"
      busy={false}
      queues={[]}
      defaultMemberId="builder"
      t={t}
      onTarget={vi.fn()}
      onMode={vi.fn()}
      onEditQueued={vi.fn()}
      onSubmit={onSubmit}
      onCancel={vi.fn().mockResolvedValue(undefined)}
    />,
  );
  return { ...utils, onSubmit, client };
}

describe("Composer completion popup", () => {
  beforeEach(() => {
    return () => setDesktopClientForTests(undefined);
  });

  it("opens the command list on a single /", async () => {
    const user = userEvent.setup();
    setup();
    const input = screen.getByLabelText("Message the team");
    await user.type(input, "/");
    const listbox = await screen.findByRole("listbox", {}, { timeout: 3000 });
    expect(listbox).toHaveTextContent("/ask");
    expect(listbox).toHaveTextContent("/new");
  });

  it("opens member mentions on a single @", async () => {
    const user = userEvent.setup();
    setup();
    const input = screen.getByLabelText("Message the team");
    await user.type(input, "@b");
    const listbox = await screen.findByRole("listbox", {}, { timeout: 3000 });
    expect(listbox).toHaveTextContent("@builder");
  });

  it("accepts a completion with Tab and replaces the token", async () => {
    const user = userEvent.setup();
    setup();
    const input = screen.getByLabelText("Message the team");
    await user.type(input, "/");
    await screen.findByRole("listbox", {}, { timeout: 3000 });
    await user.type(input, "as");
    await user.keyboard("{Tab}");
    expect(input).toHaveValue("/ask ");
  });

  it("does not show a popup for plain text", async () => {
    const user = userEvent.setup();
    setup();
    const input = screen.getByLabelText("Message the team");
    await user.type(input, "hello");
    await new Promise((resolve) => setTimeout(resolve, 250));
    expect(screen.queryByRole("listbox")).toBeNull();
  });
});
