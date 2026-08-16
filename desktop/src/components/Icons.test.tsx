import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { FolderIcon } from "./Icons";

describe("UI icons", () => {
  it("uses a high-DPI-safe geometric SVG contract", () => {
    const { container } = render(<FolderIcon />);
    const icon = container.querySelector("svg");
    expect(icon).toHaveClass("ui-icon");
    expect(icon).toHaveClass("lucide");
    expect(icon).toHaveAttribute("width", "20");
    expect(icon).toHaveAttribute("height", "20");
    expect(icon).toHaveAttribute("stroke-width", "2.16");
    expect(icon).toHaveAttribute("focusable", "false");
    expect(icon).toHaveAttribute("aria-hidden", "true");
  });
});
