import { useEffect, useRef } from "react";

const FOCUSABLE = [
  "button:not([disabled])",
  "[href]",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

/** Keep keyboard focus inside a modal and restore it when the modal closes. */
export function useDialogFocus<T extends HTMLElement>() {
  const dialog = useRef<T>(null);

  useEffect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const node = dialog.current;
    if (!node) return;
    const preferred = node.querySelector<HTMLElement>("[data-dialog-autofocus]");
    const first = preferred ?? node.querySelector<HTMLElement>(FOCUSABLE);
    first?.focus();

    const trap = (event: KeyboardEvent) => {
      if (event.key !== "Tab") return;
      const focusable = [...node.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
        (element) => element.getClientRects().length > 0 || element === document.activeElement,
      );
      if (focusable.length === 0) {
        event.preventDefault();
        node.focus();
        return;
      }
      const firstItem = focusable[0];
      const lastItem = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === firstItem) {
        event.preventDefault();
        lastItem.focus();
      } else if (!event.shiftKey && document.activeElement === lastItem) {
        event.preventDefault();
        firstItem.focus();
      }
    };
    node.addEventListener("keydown", trap);
    return () => {
      node.removeEventListener("keydown", trap);
      previous?.focus();
    };
  }, []);

  return dialog;
}
