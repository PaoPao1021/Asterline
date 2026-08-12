import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from "vitest";
import { App } from "./App";
import { setDesktopClientForTests } from "./bridge/client";
import { createMockDesktopClient } from "./bridge/mock";
import type { DesktopCommandV1 } from "./bridge/types";

describe("Asterline desktop workspace", () => {
  let client: ReturnType<typeof createMockDesktopClient>;
  let dispatchSpy: MockInstance<(command: DesktopCommandV1) => Promise<void>>;

  beforeEach(() => {
    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: 1280 });
    localStorage.setItem("asterline.locale", "en-US");
    localStorage.setItem("asterline.theme", "light");
    client = createMockDesktopClient();
    dispatchSpy = vi.spyOn(client, "dispatchDesktopCommand");
    setDesktopClientForTests(client);
  });

  afterEach(() => setDesktopClientForTests(undefined));

  it("sends a message and renders the fake runtime stream", async () => {
    const user = userEvent.setup();
    render(<App />);
    const composer = await screen.findByRole("textbox", { name: /Message the team/ });
    await user.type(composer, "Summarize the recovery contract");
    await user.keyboard("{Control>}{Enter}{/Control}");
    expect(await screen.findByText("Summarize the recovery contract")).toBeInTheDocument();
    expect(await screen.findByText(/I’m mapping that request/)).toBeInTheDocument();
  });

  it("requests saved conversations after the runtime becomes ready", async () => {
    render(<App />);
    await waitFor(() => expect(dispatchSpy).toHaveBeenCalledWith({ type: "request_resume" }));
  });

  it("resolves a visible approval", async () => {
    const user = userEvent.setup();
    render(<App />);
    expect(await screen.findByText("Action needs your approval")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Approve" }));
    await waitFor(() => expect(screen.queryByText("Action needs your approval")).not.toBeInTheDocument());
    expect(screen.getByText("Approval granted")).toBeInTheDocument();
  });

  it("switches theme and language without restarting the runtime", async () => {
    const user = userEvent.setup();
    render(<App />);
    const theme = await screen.findByRole("button", { name: "Theme" });
    await user.click(theme);
    expect(document.documentElement.dataset.theme).toBe("dark");
    await user.click(screen.getByRole("button", { name: "Language" }));
    expect(await screen.findByRole("button", { name: "主题" })).toBeInTheDocument();
    expect(localStorage.getItem("asterline.locale")).toBe("zh-CN");
  });

  it("offers an explicit manual update check", async () => {
    const user = userEvent.setup();
    const updateSpy = vi.spyOn(client, "checkDesktopUpdate");
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Check for updates" }));
    await waitFor(() => expect(updateSpy).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("Asterline Desktop is up to date.")).toBeInTheDocument();
  });

  it("starts narrow layouts with closed, mutually exclusive side drawers", async () => {
    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: 860 });
    const user = userEvent.setup();
    render(<App />);
    const sidebar = await screen.findByRole("complementary", { name: "Menu" });
    const inspector = screen.getByRole("complementary", { name: "Inspector" });
    expect(sidebar).toHaveClass("is-closed");
    expect(inspector).toHaveClass("is-closed");

    await user.click(screen.getByRole("button", { name: "Open navigation" }));
    expect(sidebar).toHaveClass("is-open");
    await user.click(within(screen.getByRole("main")).getByRole("button", { name: "Details" }));
    expect(sidebar).toHaveClass("is-closed");
    expect(inspector).toHaveClass("is-open");
  });
});
