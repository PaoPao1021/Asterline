import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { CheckIcon, ChevronIcon } from "./Icons";

export interface DropdownOption {
  value: string;
  label: string;
  /** Secondary text shown after the label (e.g. a member's backend). */
  hint?: string;
}

interface DropdownProps {
  value: string;
  options: DropdownOption[];
  onChange: (value: string) => void;
  /** Accessible name; also shown when the current value has no option. */
  label: string;
  disabled?: boolean;
  /** Opens the menu above the trigger (composer footer). */
  dropUp?: boolean;
  /** Trigger style variant; defaults to the field look. */
  variant?: "field" | "target" | "compact";
  className?: string;
  align?: "start" | "end";
}

/**
 * Themed single-select replacing native `<select>` controls (which render as
 * an unstyled OS popup and break the dark theme). Follows the ARIA 1.2
 * combobox pattern: full keyboard support, visible focus, `listbox`/`option`
 * semantics, and a type-ahead jump.
 */
export function Dropdown({
  value,
  options,
  onChange,
  label,
  disabled,
  dropUp,
  variant = "field",
  className,
  align = "start",
}: DropdownProps) {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const listbox = useRef<HTMLUListElement>(null);
  const menuId = useId();

  const selected = useMemo(
    () => options.find((option) => option.value === value),
    [options, value],
  );
  const display = selected?.label ?? (value || label);

  useEffect(() => {
    if (!open) return;
    setActiveIndex(Math.max(0, options.findIndex((option) => option.value === value)));
    const onPointerDown = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", onPointerDown);
    return () => window.removeEventListener("pointerdown", onPointerDown);
  }, [open, options, value]);

  useEffect(() => {
    if (!open) return;
    const node = listbox.current?.children[activeIndex] as HTMLElement | undefined;
    // jsdom (unit tests) does not implement scrollIntoView.
    node?.scrollIntoView?.({ block: "nearest" });
  }, [open, activeIndex]);

  const commit = (optionValue: string) => {
    setOpen(false);
    trigger.current?.focus();
    if (optionValue !== value) onChange(optionValue);
  };

  const openMenu = () => {
    if (disabled || options.length === 0) return;
    setOpen(true);
  };

  const onTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (disabled) return;
    if (!open) {
      if (event.key === "Enter" || event.key === " " || event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        openMenu();
      }
      return;
    }
    if (event.key === "Escape") {
      // Close only the menu; stop the event so an open modal (e.g. team
      // settings) does not close with it.
      event.preventDefault();
      event.stopPropagation();
      setOpen(false);
      trigger.current?.focus();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((index) => (index + 1) % options.length);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((index) => (index - 1 + options.length) % options.length);
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      setActiveIndex(0);
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      setActiveIndex(options.length - 1);
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      const option = options[activeIndex];
      if (option) commit(option.value);
      return;
    }
    // Type-ahead: jump to the next option whose label starts with the key.
    if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      const needle = event.key.toLowerCase();
      const start = activeIndex + 1;
      const ordered = [...options.slice(start), ...options.slice(0, start)];
      const match = ordered.find((option) => option.label.toLowerCase().startsWith(needle));
      if (match) {
        event.preventDefault();
        setActiveIndex(options.indexOf(match));
      }
    }
  };

  return (
    <div ref={root} className={`dropdown dropdown-${variant} ${className ?? ""} ${disabled ? "is-disabled" : ""}`}>
      <button
        ref={trigger}
        type="button"
        role="combobox"
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-controls={open ? menuId : undefined}
        aria-activedescendant={open ? `${menuId}-option-${activeIndex}` : undefined}
        aria-label={label}
        disabled={disabled}
        onClick={() => (open ? setOpen(false) : openMenu())}
        onKeyDown={onTriggerKeyDown}
      >
        <span className="dropdown-value">{display}</span>
        <ChevronIcon size={13} className={`dropdown-chevron ${open ? "is-open" : ""}`} />
      </button>
      {open && (
        <ul
          ref={listbox}
          id={menuId}
          role="listbox"
          aria-label={label}
          className={`dropdown-menu ${dropUp ? "is-dropup" : ""} ${align === "end" ? "is-end" : ""}`}
          tabIndex={-1}
        >
          {options.map((option, index) => (
            <li
              key={option.value}
              id={`${menuId}-option-${index}`}
              role="option"
              aria-selected={option.value === value}
              className={`dropdown-option ${index === activeIndex ? "is-active" : ""} ${option.value === value ? "is-selected" : ""}`}
              onPointerMove={() => setActiveIndex(index)}
              onMouseDown={(event) => {
                event.preventDefault();
                commit(option.value);
              }}
            >
              <span className="dropdown-option-label">{option.label}</span>
              {option.hint && <span className="dropdown-option-hint">{option.hint}</span>}
              {option.value === value && <CheckIcon size={13} className="dropdown-option-check" />}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
