import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from "vitest";
import { App } from "./App";
import { setDesktopClientForTests } from "./bridge/client";
import { createMockDesktopClient } from "./bridge/mock";
import type { DesktopCommandV2 } from "./bridge/types";

describe("Asterline desktop workspace", () => {
  let client: ReturnType<typeof createMockDesktopClient>;
  let dispatchSpy: MockInstance<(command: DesktopCommandV2) => Promise<void>>;

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

  it("opens only the trusted release returned by the native update check", async () => {
    const user = userEvent.setup();
    vi.spyOn(client, "checkDesktopUpdate").mockResolvedValue({
      current_version: "0.2.0",
      available_version: "0.3.0",
      release_url: "https://github.com/song0705/Asterline/releases/tag/desktop-v0.3.0",
      update_available: true,
    });
    const openSpy = vi.spyOn(client, "openDesktopUpdate");
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Check for updates" }));
    expect(await screen.findByRole("dialog", { name: "Asterline update available" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Open official download" }));

    await waitFor(() => expect(openSpy).toHaveBeenCalledWith(
      "https://github.com/song0705/Asterline/releases/tag/desktop-v0.3.0",
    ));
    expect(screen.queryByRole("dialog", { name: "Asterline update available" })).not.toBeInTheDocument();
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

  it("can restore both side panels after collapsing them on desktop", async () => {
    const user = userEvent.setup();
    render(<App />);
    const sidebar = await screen.findByRole("complementary", { name: "Menu" });
    const inspector = screen.getByRole("complementary", { name: "Inspector" });

    await user.click(within(sidebar).getByRole("button", { name: "Collapse sidebar" }));
    expect(sidebar).toHaveClass("is-closed");
    const restoreSidebar = within(screen.getByRole("main")).getByRole("button", {
      name: "Open navigation",
    });
    expect(restoreSidebar).toHaveClass("panel-restore-button");
    await user.click(restoreSidebar);
    expect(sidebar).toHaveClass("is-open");

    await user.click(within(inspector).getByRole("button", { name: "Details" }));
    expect(inspector).toHaveClass("is-closed");
    const restoreInspector = within(screen.getByRole("main")).getByRole("button", {
      name: "Details",
    });
    expect(restoreInspector).toHaveClass("panel-restore-button");
    await user.click(restoreInspector);
    expect(inspector).toHaveClass("is-open");
  });

  it("opens the utility drawer and loads bounded demo data", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Logs" }));
    expect(await screen.findByRole("complementary", { name: "Utilities" })).toBeInTheDocument();
    expect(await screen.findByText("Desktop demo log stream is ready.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Diff" }));
    expect(await screen.findByText(/utility drawer enabled/)).toBeInTheDocument();
  });

  it("exports a local diagnostics report from the logs drawer", async () => {
    const user = userEvent.setup();
    const exportSpy = vi.spyOn(client, "exportDesktopDiagnostics");
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Logs" }));
    await user.click(await screen.findByRole("button", { name: "Export diagnostics" }));
    await waitFor(() => expect(exportSpy).toHaveBeenCalledTimes(1));
    expect(await screen.findByText(/Diagnostics exported to/)).toBeInTheDocument();
  });
});
