import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { FolderIcon, PlusIcon } from "./Icons";

describe("UI icons", () => {
  it("uses Lucide's single stroke compensation with accessible SVG semantics", () => {
    const { container } = render(<FolderIcon />);
    const icon = container.querySelector("svg");
    expect(icon).toHaveClass("ui-icon");
    expect(icon).toHaveClass("lucide");
    expect(icon).toHaveAttribute("width", "20");
    expect(icon).toHaveAttribute("height", "20");
    expect(icon).toHaveAttribute("stroke-width", "2.4");
    expect(icon).toHaveAttribute("focusable", "false");
    expect(icon).toHaveAttribute("aria-hidden", "true");
  });

  it.each([12, 13, 14, 15, 16, 17, 18, 20, 24])("keeps size %i readable with a 2px rendered stroke", (size) => {
    const { container } = render(<PlusIcon size={size} />);
    const icon = container.querySelector("svg")!;
    const width = Number(icon.getAttribute("width"));
    expect(width).toBeGreaterThanOrEqual(16);
    expect(width % 2).toBe(0);
    // The SVG viewBox is 24px; normal path scaling yields 2 CSS px.
    expect(Number(icon.getAttribute("stroke-width")) * width / 24).toBeCloseTo(2);
  });

  it("preserves an explicitly requested stroke width", () => {
    const { container } = render(<PlusIcon size={16} strokeWidth={1.5} />);
    expect(container.querySelector("svg")).toHaveAttribute("stroke-width", "2.25");
  });
});
