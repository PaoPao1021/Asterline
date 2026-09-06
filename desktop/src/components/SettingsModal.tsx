import { useEffect, useMemo, useState } from "react";
import { getDesktopClient } from "../bridge/client";
import type {
  ApprovalPolicyV2,
  BackendAvailability,
  BackendKind,
  ModesConfigV2,
  ModelSummary,
  NativeSessionSummary,
  PermissionMode,
  SandboxPolicy,
  TeamMemberSettingsV2,
  TeamSettingsV2,
} from "../bridge/types";
import type { Translate } from "../i18n";
import { Dropdown } from "./Dropdown";
import { AlertIcon, ApprovalIcon, CheckIcon, ModeIcon, PlusIcon, RefreshIcon, SettingsIcon, UsersIcon, XIcon } from "./Icons";

type SettingsTab = "general" | "members" | "approvals" | "modes";

interface SettingsModalProps {
  settings: TeamSettingsV2;
  busy: boolean;
  t: Translate;
  onClose: () => void;
  onSave: (settings: TeamSettingsV2) => Promise<void>;
}

const clone = <T,>(value: T): T => structuredClone(value);

export function patchMember(member: TeamMemberSettingsV2, patch: Partial<TeamMemberSettingsV2>): TeamMemberSettingsV2 {
  // Deliberately merge into the source member: native session ids, injected
  // prompts, and future bridge fields survive edits made by older frontends.
  return { ...member, ...patch };
}

export function replaceMemberReference(settings: TeamSettingsV2, oldId: string, newId: string | null): TeamSettingsV2 {
  const next = clone(settings);
  const map = (value?: string | null) => value === oldId ? newId : value;
  if (next.default_target?.type === "member" && next.default_target.member === oldId) {
    next.default_target = newId ? { type: "member", member: newId } : null;
  }
  if (next.modes.review) {
    next.modes.review.builder = map(next.modes.review.builder);
    next.modes.review.reviewer = map(next.modes.review.reviewer);
  }
  if (next.modes.plan) {
    next.modes.plan.leader = map(next.modes.plan.leader);
    next.modes.plan.builder = map(next.modes.plan.builder);
    next.modes.plan.reviewer = map(next.modes.plan.reviewer);
  }
  if (next.modes.team) next.modes.team.coordinator = map(next.modes.team.coordinator);
  if (next.modes.brainstorm?.participants) {
    next.modes.brainstorm.participants = [...new Set(next.modes.brainstorm.participants.flatMap((id) => id === oldId ? (newId ? [newId] : []) : [id]))];
  }
  return next;
}

function newMember(index: number): TeamMemberSettingsV2 {
  return {
    id: `member-${index}`,
    display_name: `Member ${index}`,
    backend: "codex",
    role: "team member",
    cwd: null,
    model: null,
    system_prompt: null,
    sandbox: "workspace-write",
    permission_mode: null,
    allowed_tools: [],
    session_policy: "resume",
    session_id: null,
    effort: "high",
  };
}

function validate(settings: TeamSettingsV2): boolean {
  if (!settings.name.trim() || settings.members.length === 0 || settings.max_auto_relays < 1) return false;
  const ids = new Set<string>();
  const names = new Set<string>();
  const membersValid = settings.members.every((member) => {
    const id = member.id.trim().toLowerCase();
    const name = member.display_name.trim().toLowerCase();
    if (!id || !name || !/^[a-z0-9_-]+$/i.test(id) || ids.has(id) || names.has(name)) return false;
    const effortValid = member.effort == null
      || (member.backend === "agy"
        ? ["low", "medium", "high"].includes(member.effort)
        : member.backend === "codex" || member.effort !== "ultra");
    if (!effortValid) return false;
    ids.add(id);
    names.add(name);
    return true;
  });
  if (!membersValid) return false;
  const known = new Set(settings.members.map(({ id }) => id));
  const references = [
    settings.default_target?.type === "member" ? settings.default_target.member : null,
    settings.modes.review?.builder,
    settings.modes.review?.reviewer,
    settings.modes.plan?.leader,
    settings.modes.plan?.builder,
    settings.modes.plan?.reviewer,
    settings.modes.team?.coordinator,
    ...(settings.modes.brainstorm?.participants ?? []),
  ].filter((value): value is string => Boolean(value));
  const brainstorm = settings.modes.brainstorm;
  const modeLimitsValid = (settings.modes.review?.max_iterations ?? 1) >= 1
    && (settings.modes.plan?.max_iterations ?? 1) >= 1
    && (settings.modes.team?.max_iterations ?? 1) >= 1
    && (brainstorm?.generation_rounds ?? 2) >= 2
    && (brainstorm?.ideas_per_round ?? 3) >= 3;
  const brainstormParticipantsValid = !brainstorm?.participants
    || new Set(brainstorm.participants).size >= 2;
  return references.every((id) => known.has(id)) && modeLimitsValid && brainstormParticipantsValid;
}

function effortsFor(backend: BackendKind): TeamMemberSettingsV2["effort"][] {
  if (backend === "agy") return ["low", "medium", "high"];
  if (backend === "codex") return ["low", "medium", "high", "xhigh", "max", "ultra"];
  return ["low", "medium", "high", "xhigh", "max"];
}

function Field({ label, help, children, wide }: { label: string; help?: string; children: React.ReactNode; wide?: boolean }) {
  return <label className={`settings-field ${wide ? "field-wide" : ""}`}><span>{label}</span>{children}{help && <small>{help}</small>}</label>;
}

function SelectMember({ label, value, members, onChange }: { label: string; value?: string | null; members: TeamMemberSettingsV2[]; onChange: (value: string | null) => void }) {
  return (
    <Field label={label}>
      <Dropdown
        label={label}
        value={value ?? ""}
        onChange={onChange}
        options={[{ value: "", label: "Auto" }, ...members.map((member) => ({ value: member.id, label: member.display_name, hint: member.backend }))]}
      />
    </Field>
  );
}

function GeneralSettings({ draft, setDraft, t }: { draft: TeamSettingsV2; setDraft: React.Dispatch<React.SetStateAction<TeamSettingsV2>>; t: Translate }) {
  const [availability, setAvailability] = useState<BackendAvailability | null>(null);
  useEffect(() => {
    let alive = true;
    getDesktopClient().getBackendAvailability()
      .then((value) => { if (alive) setAvailability(value); })
      .catch(() => undefined);
    return () => { alive = false; };
  }, []);
  return (
    <div className="settings-pane">
      <div className="pane-intro"><h3>{t("general")}</h3><p>{t("teamNameHelp")}</p></div>
      <div className="settings-grid">
        <Field label={t("teamName")} help={t("teamNameHelp")} wide><input value={draft.name} onChange={(event) => setDraft((value) => ({ ...value, name: event.target.value }))} /></Field>
        <Field label={t("workspacePath")} wide><input value={draft.workspace} disabled title={draft.workspace} /></Field>
        <Field label={t("relayLimit")} help={t("relayHelp")}><input type="number" min={1} max={100} value={draft.max_auto_relays} onChange={(event) => setDraft((value) => ({ ...value, max_auto_relays: Number(event.target.value) }))} /></Field>
        <Field label={t("defaultRecipient")}>
          <Dropdown
            label={t("defaultRecipient")}
            value={draft.default_target?.type === "all" ? "all" : draft.default_target?.member ?? ""}
            onChange={(next) => setDraft((value) => ({ ...value, default_target: next === "all" ? { type: "all" } : next ? { type: "member", member: next } : null }))}
            options={[
              { value: "", label: t("defaultTarget") },
              { value: "all", label: t("allMembers") },
              ...draft.members.map((member) => ({ value: member.id, label: member.display_name, hint: member.backend })),
            ]}
          />
        </Field>
        <Field label={t("cliDetection")} wide>
          <div className="cli-detection" role="status">
            {(["codex", "claude", "grok", "agy"] as const).map((backend) => (
              <span key={backend} className={`cli-chip ${availability ? (availability[backend] ? "found" : "missing") : ""}`}>
                {backend} · {availability ? (availability[backend] ? t("detected") : t("notDetected")) : "…"}
              </span>
            ))}
          </div>
        </Field>
      </div>
    </div>
  );
}

function MembersSettings({ draft, setDraft, t }: { draft: TeamSettingsV2; setDraft: React.Dispatch<React.SetStateAction<TeamSettingsV2>>; t: Translate }) {
  const [selected, setSelected] = useState(draft.members[0]?.id ?? "");
  const [models, setModels] = useState<ModelSummary[]>([]);
  const [nativeSessions, setNativeSessions] = useState<NativeSessionSummary[]>([]);
  const memberIndex = Math.max(0, draft.members.findIndex(({ id }) => id === selected));
  const member = draft.members[memberIndex];
  const update = (patch: Partial<TeamMemberSettingsV2>) => setDraft((value) => ({ ...value, members: value.members.map((item, index) => index === memberIndex ? patchMember(item, patch) : item) }));
  const refreshCatalog = async () => {
    if (!member) return;
    try {
      const [modelList, sessionList] = await Promise.all([
        getDesktopClient().listModels(member.backend, draft.workspace).catch(() => [] as ModelSummary[]),
        getDesktopClient().listNativeSessions(member.backend, draft.workspace).catch(() => [] as NativeSessionSummary[]),
      ]);
      setModels(modelList);
      setNativeSessions(sessionList);
    } catch { /* catalog discovery is best-effort; manual entry still works. */ }
  };
  const remove = () => {
    if (draft.members.length <= 1) return;
    const nextMembers = draft.members.filter((_, index) => index !== memberIndex);
    setDraft((value) => replaceMemberReference({ ...value, members: nextMembers }, member.id, null));
    setSelected(nextMembers[0].id);
  };
  if (!member) return null;
  return (
    <div className="members-settings">
      <aside className="settings-member-list">
        <div className="settings-list-heading"><strong>{t("members")}</strong><button onClick={() => { const item = newMember(draft.members.length + 1); setDraft((value) => ({ ...value, members: [...value.members, item] })); setSelected(item.id); }}><PlusIcon size={14} />{t("addMember")}</button></div>
        {draft.members.map((item) => <button key={item.id} className={member === item ? "active" : ""} onClick={() => setSelected(item.id)}><span className={`backend-dot backend-${item.backend}`} /><span><strong>{item.display_name}</strong><small>@{item.id} · {item.backend}</small></span></button>)}
      </aside>
      <div className="settings-pane member-editor">
        <div className="pane-intro member-editor-head"><div><h3>{member.display_name}</h3><p>@{member.id} · {member.backend}</p></div><button className="danger-text-button" onClick={remove} disabled={draft.members.length <= 1}>{t("removeMember")}</button></div>
        <div className="settings-grid">
          <Field label={t("displayName")}><input aria-label={t("displayName")} value={member.display_name} onChange={(event) => update({ display_name: event.target.value })} /></Field>
          <Field label={t("memberId")}><input aria-label={t("memberId")} value={member.id} onChange={(event) => { const id = event.target.value; setDraft((value) => replaceMemberReference({ ...value, members: value.members.map((item, index) => index === memberIndex ? patchMember(item, { id }) : item) }, member.id, id)); setSelected(id); }} /></Field>
          <Field label={t("backend")}><Dropdown label={t("backend")} value={member.backend} onChange={(next) => { const backend = next as BackendKind; const effort = effortsFor(backend).includes(member.effort) ? member.effort : null; update({ backend, effort }); }} options={["codex", "claude", "grok", "agy"].map((value) => ({ value, label: value }))} /></Field>
          <Field label={t("role")}><input value={member.role} onChange={(event) => update({ role: event.target.value })} /></Field>
          <Field label={t("model")} help={t("manualModel")}><input list={`models-${member.id}`} value={member.model ?? ""} placeholder="default" onChange={(event) => update({ model: event.target.value || null })} /><datalist id={`models-${member.id}`}>{models.map((model) => <option key={model.id} value={model.id}>{model.description ?? model.name}</option>)}</datalist></Field>
          <Field label={t("modelCatalog")}><button type="button" className="secondary-button" onClick={() => void refreshCatalog()}><RefreshIcon size={13} />{t("refreshModels")}</button><small>{t("refreshModelsHint")}</small></Field>
          <Field label={t("effort")}><Dropdown label={t("effort")} value={member.effort ?? ""} onChange={(next) => update({ effort: (next || null) as TeamMemberSettingsV2["effort"] })} options={[{ value: "", label: "default" }, ...effortsFor(member.backend).map((value) => ({ value: String(value), label: String(value) }))]} /></Field>
          <Field label={t("sandbox")}><Dropdown label={t("sandbox")} value={member.sandbox} onChange={(next) => update({ sandbox: next as SandboxPolicy })} options={["read-only", "workspace-write", "danger-full-access"].map((value) => ({ value, label: value }))} /></Field>
          <Field label={t("permissionMode")}><Dropdown label={t("permissionMode")} value={member.permission_mode ?? ""} onChange={(next) => update({ permission_mode: (next || null) as PermissionMode | null })} options={[{ value: "", label: "default" }, ...["default", "acceptEdits", "plan", "auto", "dontAsk", "bypassPermissions"].map((value) => ({ value, label: value }))]} /></Field>
          <Field label={t("cwd")} wide><input value={member.cwd ?? ""} placeholder={draft.workspace} onChange={(event) => update({ cwd: event.target.value || null })} /></Field>
          <Field label={t("allowedTools")} help={t("allowedToolsHelp")} wide><input value={member.allowed_tools.join(", ")} onChange={(event) => update({ allowed_tools: event.target.value.split(",").map((value) => value.trim()).filter(Boolean) })} /></Field>
          <Field label={t("sessionPolicy")}><Dropdown label={t("sessionPolicy")} value={member.session_policy} onChange={(next) => update({ session_policy: next as "resume" | "fresh" })} options={[{ value: "resume", label: "resume" }, { value: "fresh", label: "fresh" }]} /></Field>
          <Field label={t("sessionId")} help={t("nativeSessionSearch")}><input list={`sessions-${member.id}`} value={member.session_id ?? ""} placeholder={t("sessionId")} onChange={(event) => update({ session_id: event.target.value || null })} /><datalist id={`sessions-${member.id}`}>{nativeSessions.map((session) => <option key={session.session_id} value={session.session_id}>{session.preview || session.session_id}</option>)}</datalist></Field>
          <Field label={t("systemPrompt")} wide><textarea rows={5} value={member.system_prompt ?? ""} onChange={(event) => update({ system_prompt: event.target.value || null })} /></Field>
        </div>
      </div>
    </div>
  );
}

function keywordText(keywords: Record<string, string[]>): string {
  return Object.entries(keywords).map(([name, values]) => `${name}: ${values.join(", ")}`).join("\n");
}

function parseKeywords(value: string): Record<string, string[]> {
  const result: Record<string, string[]> = {};
  value.split("\n").forEach((line) => {
    const [name, rest] = line.split(":", 2);
    if (name?.trim() && rest?.trim()) result[name.trim()] = rest.split(",").map((word) => word.trim()).filter(Boolean);
  });
  return result;
}

function ApprovalsSettings({ policy, onChange, t }: { policy: ApprovalPolicyV2; onChange: (policy: ApprovalPolicyV2) => void; t: Translate }) {
  const toggle = <T extends string,>(values: T[] | null | undefined, value: T, defaults: T[]): T[] => {
    const current = values ?? defaults;
    return current.includes(value) ? current.filter((item) => item !== value) : [...current, value];
  };
  const gates = policy.gate ?? ["git", "shell", "file"];
  const surfaces = policy.apply_to ?? ["user", "relay", "mode"];
  return (
    <div className="settings-pane">
      <div className="pane-intro"><h3>{t("approvals")}</h3><p>{t("gateHelp")}</p></div>
      <div className="settings-block"><strong>{t("gateCategories")}</strong><div className="check-grid">{["git", "shell", "file"].map((gate) => <label key={gate}><input type="checkbox" checked={gates.includes(gate)} onChange={() => onChange({ ...policy, gate: toggle(policy.gate, gate, ["git", "shell", "file"]) })} /><span><CheckIcon size={12} /></span>{gate}</label>)}</div></div>
      <div className="settings-block"><strong>{t("surfaces")}</strong><div className="check-grid">{(["user", "relay", "mode"] as const).map((surface) => <label key={surface}><input type="checkbox" checked={surfaces.includes(surface)} onChange={() => onChange({ ...policy, apply_to: toggle(policy.apply_to, surface, ["user", "relay", "mode"]) })} /><span><CheckIcon size={12} /></span>{surface}</label>)}</div></div>
      <Field label={t("customKeywords")} help={t("keywordHelp")} wide><textarea rows={7} value={keywordText(policy.keywords)} placeholder="deploy: publish, release" onChange={(event) => onChange({ ...policy, keywords: parseKeywords(event.target.value) })} /></Field>
    </div>
  );
}

function ModeCard({ title, children }: { title: string; children: React.ReactNode }) {
  return <section className="mode-settings-card"><div className="mode-settings-title"><span /><strong>{title}</strong></div><div className="settings-grid compact">{children}</div></section>;
}

function ModesSettings({ modes, members, onChange, t }: { modes: ModesConfigV2; members: TeamMemberSettingsV2[]; onChange: (modes: ModesConfigV2) => void; t: Translate }) {
  const review = modes.review ?? {};
  const plan = modes.plan ?? {};
  const brainstorm = modes.brainstorm ?? {};
  const team = modes.team ?? {};
  return (
    <div className="settings-pane modes-pane">
      <div className="pane-intro"><h3>{t("modes")}</h3><p>{t("relayHelp")}</p></div>
      <ModeCard title={t("review")}>
        <SelectMember label={t("builder")} value={review.builder} members={members} onChange={(builder) => onChange({ ...modes, review: { ...review, builder } })} />
        <SelectMember label={t("reviewer")} value={review.reviewer} members={members} onChange={(reviewer) => onChange({ ...modes, review: { ...review, reviewer } })} />
        <Field label={t("maxIterations")}><input type="number" min={1} value={review.max_iterations ?? 3} onChange={(event) => onChange({ ...modes, review: { ...review, max_iterations: Number(event.target.value) } })} /></Field>
        <Field label={t("verifyCommand")}><input value={review.verify_command ?? ""} onChange={(event) => onChange({ ...modes, review: { ...review, verify_command: event.target.value || null } })} /></Field>
        <label className="switch-field"><input type="checkbox" checked={review.auto_verify ?? true} onChange={(event) => onChange({ ...modes, review: { ...review, auto_verify: event.target.checked } })} /><span />{t("autoVerify")}</label>
      </ModeCard>
      <ModeCard title={t("plan")}>
        <SelectMember label={t("leader")} value={plan.leader} members={members} onChange={(leader) => onChange({ ...modes, plan: { ...plan, leader } })} />
        <SelectMember label={t("planBuilder")} value={plan.builder} members={members} onChange={(builder) => onChange({ ...modes, plan: { ...plan, builder } })} />
        <SelectMember label={t("reviewer")} value={plan.reviewer} members={members} onChange={(reviewer) => onChange({ ...modes, plan: { ...plan, reviewer } })} />
        <Field label={t("maxIterations")}><input type="number" min={1} value={plan.max_iterations ?? 3} onChange={(event) => onChange({ ...modes, plan: { ...plan, max_iterations: Number(event.target.value) } })} /></Field>
        <label className="switch-field"><input type="checkbox" checked={plan.auto_execute ?? false} onChange={(event) => onChange({ ...modes, plan: { ...plan, auto_execute: event.target.checked } })} /><span />{t("planAutoExecute")}</label>
        <Field label={t("verifyCommand")}><input value={plan.verify_command ?? ""} onChange={(event) => onChange({ ...modes, plan: { ...plan, verify_command: event.target.value || null } })} /></Field>
        <label className="switch-field"><input type="checkbox" checked={plan.auto_verify ?? true} onChange={(event) => onChange({ ...modes, plan: { ...plan, auto_verify: event.target.checked } })} /><span />{t("autoVerify")}</label>
      </ModeCard>
      <ModeCard title={t("brainstorm")}>
        <Field label={t("participants")} wide><div className="participant-grid">{members.map((member) => { const selected = (brainstorm.participants ?? members.map(({ id }) => id)).includes(member.id); return <label key={member.id}><input type="checkbox" checked={selected} onChange={() => { const current = brainstorm.participants ?? members.map(({ id }) => id); const participants = selected ? current.filter((id) => id !== member.id) : [...current, member.id]; onChange({ ...modes, brainstorm: { ...brainstorm, participants } }); }} /><span><CheckIcon size={11} /></span>{member.display_name}</label>; })}</div></Field>
        <Field label={t("generationRounds")}><input type="number" min={2} value={brainstorm.generation_rounds ?? 3} onChange={(event) => onChange({ ...modes, brainstorm: { ...brainstorm, generation_rounds: Number(event.target.value) } })} /></Field>
        <Field label={t("ideasPerRound")}><input type="number" min={3} value={brainstorm.ideas_per_round ?? 4} onChange={(event) => onChange({ ...modes, brainstorm: { ...brainstorm, ideas_per_round: Number(event.target.value) } })} /></Field>
      </ModeCard>
      <ModeCard title={t("team")}>
        <SelectMember label={t("coordinator")} value={team.coordinator} members={members} onChange={(coordinator) => onChange({ ...modes, team: { ...team, coordinator } })} />
        <Field label={t("maxIterations")}><input type="number" min={1} value={team.max_iterations ?? 3} onChange={(event) => onChange({ ...modes, team: { ...team, max_iterations: Number(event.target.value) } })} /></Field>
        <Field label={t("verifyCommand")} wide><input value={team.verify_command ?? ""} onChange={(event) => onChange({ ...modes, team: { ...team, verify_command: event.target.value || null } })} /></Field>
        <label className="switch-field"><input type="checkbox" checked={team.auto_verify ?? true} onChange={(event) => onChange({ ...modes, team: { ...team, auto_verify: event.target.checked } })} /><span />{t("autoVerify")}</label>
      </ModeCard>
    </div>
  );
}

export function SettingsModal({ settings, busy, t, onClose, onSave }: SettingsModalProps) {
  const [draft, setDraft] = useState(() => clone(settings));
  const [tab, setTab] = useState<SettingsTab>("general");
  const valid = useMemo(() => validate(draft), [draft]);
  useEffect(() => setDraft(clone(settings)), [settings]);
  useEffect(() => {
    const listener = (event: KeyboardEvent) => { if (event.key === "Escape" && !busy) onClose(); };
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, [busy, onClose]);
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) onClose(); }}>
      <section className="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title">
        <header className="settings-header"><div><span className="settings-header-icon"><SettingsIcon /></span><div><h2 id="settings-title">{t("settings")}</h2><p>{draft.name}</p></div></div><button className="icon-button" onClick={onClose} disabled={busy} aria-label={t("close")}><XIcon /></button></header>
        <div className="settings-body">
          <nav className="settings-nav" aria-label={t("settings")}>
            {(["general", "members", "approvals", "modes"] as SettingsTab[]).map((value) => <button key={value} className={tab === value ? "active" : ""} onClick={() => setTab(value)}>{value === "members" ? <UsersIcon size={16} /> : value === "approvals" ? <ApprovalIcon size={16} /> : value === "modes" ? <ModeIcon size={16} /> : <SettingsIcon size={16} />}{t(value === "members" ? "memberSettings" : value)}</button>)}
          </nav>
          <main className="settings-content">
            {tab === "general" && <GeneralSettings draft={draft} setDraft={setDraft} t={t} />}
            {tab === "members" && <MembersSettings draft={draft} setDraft={setDraft} t={t} />}
            {tab === "approvals" && <ApprovalsSettings policy={draft.approvals} onChange={(approvals) => setDraft((value) => ({ ...value, approvals }))} t={t} />}
            {tab === "modes" && <ModesSettings modes={draft.modes} members={draft.members} onChange={(modes) => setDraft((value) => ({ ...value, modes }))} t={t} />}
          </main>
        </div>
        <footer className="settings-footer">{!valid && <span className="validation-message"><AlertIcon size={14} />{t("invalidTeam")}</span>}<button className="secondary-button" onClick={onClose} disabled={busy}>{t("cancel")}</button><button className="primary-button" disabled={!valid || busy} onClick={() => void onSave(clone(draft))}>{busy ? t("saving") : t("save")}</button></footer>
      </section>
    </div>
  );
}
