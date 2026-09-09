import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type ClipboardEvent,
  type DragEvent,
  type KeyboardEvent,
} from "react";
import { bytesToBase64, getDesktopClient } from "../bridge/client";
import type {
  Completion,
  MemberQueueV2,
  MemberSummaryV2,
  StagedAttachment,
  TerminalMode,
} from "../bridge/types";
import type { Translate } from "../i18n";
import { Dropdown } from "./Dropdown";
import { ImageIcon, PlusIcon, SendIcon, SparkIcon, StopIcon, XIcon } from "./Icons";

/** Composer budget mirrored from the shared contract (256 KiB). */
export const MAX_COMPOSER_BYTES = 256 * 1024;
/** Image-per-message cap mirrored from the shared adapter. */
export const MAX_PROMPT_IMAGES = 4;
/** Prompt-history caps mirrored from the TUI app state (1 000 entries / 1 MiB). */
const MAX_HISTORY_ITEMS = 1_000;
const MAX_HISTORY_BYTES = 1024 * 1024;

/** Drop oldest entries beyond the TUI's prompt-history caps. */
function trimHistory(entries: string[]): string[] {
  let next = entries.slice(-MAX_HISTORY_ITEMS);
  let total = next.reduce((sum, entry) => sum + entry.length, 0);
  while (next.length > 0 && total > MAX_HISTORY_BYTES) {
    total -= next[0].length;
    next = next.slice(1);
  }
  return next;
}

const MODES: TerminalMode[] = ["normal", "review", "plan", "brainstorm", "team"];

interface ComposerProps {
  members: MemberSummaryV2[];
  mode: TerminalMode;
  target: string;
  busy: boolean;
  disabled?: boolean;
  queues: MemberQueueV2[];
  /** Member receiving "default target" traffic (team default_target). */
  defaultMemberId?: string | null;
  /** App-pushed text (queued prompt pull-back); applied when `nonce` changes. */
  injection?: { text: string; nonce: number } | null;
  t: Translate;
  onTarget: (target: string) => void;
  onMode: (mode: TerminalMode) => void;
  onEditQueued: () => void;
  onSubmit: (text: string, attachments: StagedAttachment[]) => Promise<boolean>;
  onCancel: () => Promise<void>;
}

export function Composer({
  members,
  mode,
  target,
  busy,
  disabled,
  queues,
  defaultMemberId,
  injection,
  t,
  onTarget,
  onMode,
  onEditQueued,
  onSubmit,
  onCancel,
}: ComposerProps) {
  const [text, setText] = useState("");
  const [attachments, setAttachments] = useState<StagedAttachment[]>([]);
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState<number | null>(null);
  const [historySearch, setHistorySearch] = useState<string | null>(null);
  const [completion, setCompletion] = useState<Completion | null>(null);
  const [completionIndex, setCompletionIndex] = useState(0);
  const [notice, setNotice] = useState<string | null>(null);
  const textarea = useRef<HTMLTextAreaElement>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const completionRequest = useRef(0);

  // Drafts are kept per target so switching recipients never loses text.
  const drafts = useRef(new Map<string, string>());

  useEffect(() => {
    const listener = (event: KeyboardEvent | globalThis.KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "l") textarea.current?.focus();
    };
    window.addEventListener("keydown", listener as EventListener);
    return () => window.removeEventListener("keydown", listener as EventListener);
  }, []);

  // Queued prompt pull-back and other app-pushed text.
  useEffect(() => {
    if (injection?.text) {
      setText(injection.text);
      requestAnimationFrame(() => {
        const element = textarea.current;
        if (element) {
          element.style.height = "auto";
          element.style.height = `${Math.min(element.scrollHeight, 170)}px`;
          element.focus();
        }
      });
    }
  }, [injection?.nonce, injection?.text]);

  const autoGrow = () => {
    const element = textarea.current;
    if (!element) return;
    element.style.height = "auto";
    element.style.height = `${Math.min(element.scrollHeight, 170)}px`;
  };

  const queue = useMemo(
    () =>
      queues.find((entry) => entry.member === target)
      ?? (target === "default" && defaultMemberId
        ? queues.find((entry) => entry.member === defaultMemberId)
        : undefined),
    [queues, target, defaultMemberId],
  );

  const addAttachment = (staged: StagedAttachment) => {
    setAttachments((current) => {
      if (current.length >= MAX_PROMPT_IMAGES) {
        setNotice(t("addImageHint", { count: MAX_PROMPT_IMAGES }));
        return current;
      }
      return [...current, staged];
    });
  };

  const stageFiles = async (files: File[]) => {
    const client = getDesktopClient();
    for (const file of files.slice(0, MAX_PROMPT_IMAGES)) {
      try {
        const bytes = new Uint8Array(await file.arrayBuffer());
        addAttachment(await client.stageImageBytes(bytesToBase64(bytes)));
      } catch (error) {
        setNotice(error instanceof Error ? error.message : String(error));
      }
    }
  };

  const stageClipboard = async () => {
    const client = getDesktopClient();
    if (client.kind === "mock") {
      setNotice(t("clipboardUnavailable"));
      return;
    }
    try {
      addAttachment(await client.stageClipboardImage());
      return;
    } catch {
      // Native clipboard unavailable; fall back to the webview clipboard.
    }
    try {
      const items = await navigator.clipboard.read();
      for (const item of items) {
        const type = item.types.find((candidate) => candidate.startsWith("image/"));
        if (!type) continue;
        const blob = await item.getType(type);
        const bytes = new Uint8Array(await blob.arrayBuffer());
        addAttachment(await client.stageImageBytes(bytesToBase64(bytes)));
        return;
      }
      setNotice(t("clipboardUnavailable"));
    } catch {
      setNotice(t("clipboardUnavailable"));
    }
  };

  const onPaste = async (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const files = Array.from(event.clipboardData.files).filter((file) => file.type.startsWith("image/"));
    if (files.length === 0) return;
    event.preventDefault();
    await stageFiles(files);
  };

  const onDrop = async (event: DragEvent<HTMLTextAreaElement>) => {
    const files = Array.from(event.dataTransfer.files).filter((file) => file.type.startsWith("image/"));
    if (files.length === 0) return;
    event.preventDefault();
    await stageFiles(files);
  };

  const acceptCompletion = (insert: string, tokenStart: number) => {
    const element = textarea.current;
    const cursor = element?.selectionStart ?? text.length;
    const head = text.slice(0, cursor);
    const tail = text.slice(cursor);
    const replaced = head.slice(0, tokenStart) + insert;
    setText(replaced + tail);
    setCompletion(null);
    requestAnimationFrame(() => {
      const next = textarea.current;
      if (next) {
        next.setSelectionRange(replaced.length, replaced.length);
        next.focus();
      }
    });
  };

  const refreshCompletion = async (value: string, cursor: number | null) => {
    const head = cursor === null ? value : value.slice(0, cursor);
    // A single "/" or "@" must already open the popup, exactly like the TUI.
    // The shared completion returns None for plain text, so no further guard
    // is needed here.
    if (!head) {
      setCompletion(null);
      return;
    }
    const requestId = ++completionRequest.current;
    try {
      const found = await getDesktopClient().completeComposer(head);
      if (completionRequest.current === requestId) {
        setCompletion(found);
        setCompletionIndex(0);
      }
    } catch {
      if (completionRequest.current === requestId) setCompletion(null);
    }
  };

  const onChange = (event: ChangeEvent<HTMLTextAreaElement>) => {
    let value = event.target.value;
    if (value.length > MAX_COMPOSER_BYTES) {
      value = value.slice(0, MAX_COMPOSER_BYTES);
      setNotice(t("draftKept"));
    }
    setText(value);
    setHistoryIndex(null);
    setNotice(null);
    void refreshCompletion(value, event.target.selectionStart);
    autoGrow();
  };

  const submit = async () => {
    if (disabled) return;
    if (!text.trim() && attachments.length === 0) return;
    const submitted = text;
    const staged = attachments;
    const accepted = await onSubmit(submitted, staged);
    if (accepted) {
      if (submitted.trim()) {
        setHistory((current) => trimHistory([...current.filter((entry) => entry !== submitted), submitted]));
      }
      // A second submission may already be in flight (fast consecutive
      // sends): only clear what this submission actually owned.
      setText((current) => (current === submitted ? "" : current));
      setAttachments((current) => (current === staged ? [] : current));
      drafts.current.set(target, "");
      setHistoryIndex(null);
      setCompletion(null);
      setNotice(null);
      requestAnimationFrame(() => {
        const element = textarea.current;
        if (element) {
          element.style.height = "auto";
          element.focus();
        }
      });
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (completion && completion.items.length > 0) {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setCompletionIndex((index) => (index + 1) % completion.items.length);
        return;
      }
      if (event.key === "ArrowUp") {
        event.preventDefault();
        setCompletionIndex((index) => (index - 1 + completion.items.length) % completion.items.length);
        return;
      }
      if (event.key === "Tab" || event.key === "Enter") {
        event.preventDefault();
        acceptCompletion(completion.items[completionIndex].insert, completion.token_start);
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        setCompletion(null);
        return;
      }
    }

    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "r") {
      event.preventDefault();
      setHistorySearch((current) => (current === null ? "" : current));
      return;
    }

    if (historySearch !== null) {
      if (event.key === "Escape") {
        event.preventDefault();
        setHistorySearch(null);
      }
      return;
    }

    // Plain Enter submits; Shift/Alt+Enter insert a newline (TUI behavior).
    if (event.key === "Enter" && !event.shiftKey && !event.altKey) {
      event.preventDefault();
      void submit();
      return;
    }

    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      const element = textarea.current;
      const cursor = element?.selectionStart ?? 0;
      const atEdge =
        (event.key === "ArrowUp" && (cursor === 0 || !text.includes("\n"))) ||
        (event.key === "ArrowDown" && (cursor === text.length || !text.includes("\n")));
      if (atEdge && history.length > 0) {
        event.preventDefault();
        const index =
          event.key === "ArrowUp"
            ? historyIndex === null
              ? history.length - 1
              : Math.max(0, historyIndex - 1)
            : historyIndex === null
              ? null
              : historyIndex + 1 >= history.length
                ? null
                : historyIndex + 1;
        setHistoryIndex(index);
        setText(index === null ? drafts.current.get(target) ?? "" : history[index]);
        requestAnimationFrame(autoGrow);
      }
    }
  };

  const historyMatches = useMemo(() => {
    if (historySearch === null) return [];
    const query = historySearch.toLowerCase();
    return [...history].reverse().filter((entry) => entry.toLowerCase().includes(query));
  }, [history, historySearch]);

  return (
    <div
      className="composer-wrap"
      onDragOver={(event) => event.preventDefault()}
      onDrop={(event) => void onDrop(event as unknown as DragEvent<HTMLTextAreaElement>)}
    >
      <div className="mode-row" role="toolbar" aria-label={t("modes")}>
        <div className="mode-switcher">
          <SparkIcon size={13} className="mode-switcher-glyph" />
          {MODES.map((value) => <button key={value} className={mode === value ? "active" : ""} aria-pressed={mode === value} onClick={() => onMode(value)} disabled={disabled}>{t(value)}</button>)}
        </div>
      </div>
      <div className={`composer ${disabled ? "disabled" : ""}`}>
        <div className="composer-main">
          <textarea
            ref={textarea}
            value={text}
            rows={1}
            disabled={disabled}
            aria-label={t("composerPlaceholder")}
            placeholder={t("composerPlaceholder")}
            onChange={onChange}
            onKeyDown={onKeyDown}
            onPaste={(event) => void onPaste(event)}
          />
        </div>
        {attachments.length > 0 && (
          <div className="composer-attachments" aria-label={t("attachments")}>
            {attachments.map((attachment) => (
              <span key={attachment.token} className="attachment-chip">
                <ImageIcon size={11} />
                {attachment.label}
                <button
                  className="subtle-icon-button"
                  aria-label={`${t("removeAttachment")}: ${attachment.label}`}
                  onClick={() => {
                    setAttachments((current) => current.filter((entry) => entry.token !== attachment.token));
                    getDesktopClient().removeStagedAttachment(attachment.token).catch(() => undefined);
                  }}
                >
                  <XIcon size={16} />
                </button>
              </span>
            ))}
          </div>
        )}
        {queue && queue.prompts.length > 0 && (
          <div className="composer-queue" role="status">
            <span>{t("queuedFor", { name: members.find((member) => member.id === queue.member)?.display_name ?? queue.member, count: queue.prompts.length })}</span>
            <button onClick={onEditQueued} disabled={disabled}>{t("editLastQueued")}</button>
          </div>
        )}
        <div className="composer-footer">
          <Dropdown
            variant="target"
            dropUp
            label={t("target")}
            value={target}
            disabled={disabled}
            onChange={onTarget}
            options={[
              { value: "default", label: t("defaultTarget") },
              { value: "all", label: t("allMembers") },
              ...members.map((member) => ({ value: member.id, label: member.display_name, hint: member.backend })),
            ]}
          />
          <button className="icon-button composer-image-button" aria-label={t("addImage")} title={t("addImageHint", { count: MAX_PROMPT_IMAGES })} disabled={disabled} onClick={() => void stageClipboard()}><ImageIcon size={15} /></button>
          <button className="icon-button composer-image-button" aria-label={t("addImage")} title={t("addImage")} disabled={disabled} onClick={() => fileInput.current?.click()}><PlusIcon size={15} /></button>
          <input
            ref={fileInput}
            type="file"
            accept="image/png,image/jpeg,image/gif,image/webp,image/tiff"
            multiple
            hidden
            onChange={(event) => {
              void stageFiles(Array.from(event.target.files ?? []));
              event.target.value = "";
            }}
          />
          <span className="command-hint">{notice ? <b className="composer-warning">{notice}</b> : text.startsWith("/") ? t("commandHint") : t("shortcut")}</span>
          {busy ? <button className="send-button stop" onClick={() => void onCancel()} title={t("stop")}><StopIcon size={16} /><span>{t("stop")}</span></button> : <button className="send-button" onClick={() => void submit()} disabled={(!text.trim() && attachments.length === 0) || disabled} title={t("send")}><SendIcon size={16} /><span>{t("send")}</span></button>}
        </div>
        {completion && completion.items.length > 0 && (
          <ul className="completion-popup" role="listbox" aria-label={completion.title}>
            {completion.items.map((item, index) => (
              <li
                key={item.insert}
                role="option"
                aria-selected={index === completionIndex}
                className={index === completionIndex ? "active" : ""}
                onMouseDown={(event) => {
                  event.preventDefault();
                  acceptCompletion(item.insert, completion.token_start);
                }}
              >
                {item.label}
              </li>
            ))}
          </ul>
        )}
      </div>
      {historySearch !== null && (
        <div className="history-search" role="dialog" aria-label={t("historySearch")}>
          <input
            autoFocus
            value={historySearch}
            placeholder={t("historySearch")}
            aria-label={t("historySearch")}
            onChange={(event) => setHistorySearch(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && historyMatches[0]) {
                setText(historyMatches[0]);
                setHistorySearch(null);
              }
            }}
          />
          <ul>
            {historyMatches.map((entry) => (
              <li key={entry}>
                <button onClick={() => { setText(entry); setHistorySearch(null); }}>{entry}</button>
              </li>
            ))}
          </ul>
          {historySearch !== "" && historyMatches.length === 0 && <p>{t("noHistoryMatches")}</p>}
        </div>
      )}
    </div>
  );
}
