import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import type { MemberSummaryV1, TerminalMode } from "../bridge/types";
import type { Translate } from "../i18n";
import { SendIcon, SparkIcon, StopIcon } from "./Icons";

const MODES: TerminalMode[] = ["normal", "review", "plan", "brainstorm", "team"];

interface ComposerProps {
  members: MemberSummaryV1[];
  mode: TerminalMode;
  target: string;
  busy: boolean;
  disabled?: boolean;
  t: Translate;
  onTarget: (target: string) => void;
  onMode: (mode: TerminalMode) => void;
  onSubmit: (text: string) => Promise<boolean>;
  onCancel: () => Promise<void>;
}

export function Composer({ members, mode, target, busy, disabled, t, onTarget, onMode, onSubmit, onCancel }: ComposerProps) {
  const [text, setText] = useState("");
  const textarea = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const listener = (event: KeyboardEvent | globalThis.KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "l") textarea.current?.focus();
    };
    window.addEventListener("keydown", listener as EventListener);
    return () => window.removeEventListener("keydown", listener as EventListener);
  }, []);

  const submit = async () => {
    if (!text.trim() || disabled) return;
    if (await onSubmit(text)) {
      setText("");
      if (textarea.current) textarea.current.style.height = "auto";
    }
  };

  const keyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      void submit();
    }
  };

  return (
    <div className="composer-wrap">
      <div className="mode-row" aria-label={t("modes")}>
        <span><SparkIcon size={14} />{t("modes")}</span>
        <div className="mode-switcher">{MODES.map((value) => <button key={value} className={mode === value ? "active" : ""} onClick={() => onMode(value)}>{t(value)}</button>)}</div>
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
            onChange={(event) => {
              setText(event.target.value);
              event.currentTarget.style.height = "auto";
              event.currentTarget.style.height = `${Math.min(event.currentTarget.scrollHeight, 170)}px`;
            }}
            onKeyDown={keyDown}
          />
        </div>
        <div className="composer-footer">
          <label className="target-select"><span>@</span><select value={target} onChange={(event) => onTarget(event.target.value)} aria-label={t("target")}><option value="default">{t("defaultTarget")}</option><option value="all">{t("allMembers")}</option>{members.map((member) => <option key={member.id} value={member.id}>{member.display_name}</option>)}</select></label>
          <span className="command-hint">{text.startsWith("/") ? t("commandHint") : t("shortcut")}</span>
          {busy ? <button className="send-button stop" onClick={() => void onCancel()} title={t("stop")}><StopIcon size={16} /><span>{t("stop")}</span></button> : <button className="send-button" onClick={() => void submit()} disabled={!text.trim() || disabled} title={t("send")}><SendIcon size={16} /><span>{t("send")}</span></button>}
        </div>
      </div>
    </div>
  );
}
