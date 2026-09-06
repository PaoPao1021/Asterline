import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Dropdown } from "./Dropdown";

const options = [
  { value: "default", label: "Default target" },
  { value: "all", label: "All members" },
  { value: "builder", label: "Builder", hint: "codex" },
];

describe("Dropdown", () => {
  it("opens on click, shows themed options with selection state, and commits", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<Dropdown label="Target" value="all" options={options} onChange={onChange} />);

    const trigger = screen.getByRole("combobox", { name: "Target" });
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    await user.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "true");

    const listbox = screen.getByRole("listbox", { name: "Target" });
    const selected = screen.getByRole("option", { name: /All members/ });
    expect(selected).toHaveAttribute("aria-selected", "true");

    await user.click(screen.getByRole("option", { name: /Builder/ }));
    expect(onChange).toHaveBeenCalledWith("builder");
    // The menu closes and focus returns to the trigger.
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(trigger).toHaveFocus();
  });

  it("supports full keyboard navigation", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<Dropdown label="Target" value="default" options={options} onChange={onChange} />);

    const trigger = screen.getByRole("combobox", { name: "Target" });
    await user.click(trigger);

    // ArrowDown moves the active option; Enter commits it.
    await user.keyboard("{ArrowDown}");
    await user.keyboard("{Enter}");
    expect(onChange).toHaveBeenCalledWith("all");

    // Escape closes without committing.
    await user.click(trigger);
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("renders the current selection as the trigger value", () => {
    render(<Dropdown label="Target" value="builder" options={options} onChange={vi.fn()} />);
    expect(screen.getByRole("combobox", { name: "Target" })).toHaveTextContent("Builder");
  });
});
