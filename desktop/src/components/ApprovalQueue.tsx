import type { ApprovalSummaryV1, MemberSummaryV1 } from "../bridge/types";
import type { Translate } from "../i18n";
import { AlertIcon, CheckIcon, ToolIcon, XIcon } from "./Icons";

interface ApprovalQueueProps {
  approvals: ApprovalSummaryV1[];
  members: MemberSummaryV1[];
  busyId?: number | null;
  t: Translate;
  onDecision: (id: number, decision: "approve" | "reject") => Promise<void>;
}

export function ApprovalQueue({ approvals, members, busyId, t, onDecision }: ApprovalQueueProps) {
  if (approvals.length === 0) return null;
  return (
    <section className="approval-queue" aria-label={t("approvals")}>
      {approvals.map((approval) => {
        const member = members.find(({ id }) => id === approval.member);
        return (
          <article className="approval-card" key={approval.id}>
            <div className="approval-accent"><AlertIcon size={18} /></div>
            <div className="approval-content">
              <div className="approval-heading"><strong>{t("approvalTitle")}</strong><span><ToolIcon size={13} />{approval.action}</span></div>
              <p>{approval.body}</p>
              <small>{t("requestedBy", { name: member?.display_name ?? approval.member ?? "runtime" })}</small>
            </div>
            <div className="approval-actions">
              <button className="reject-button" disabled={busyId === approval.id} onClick={() => void onDecision(approval.id, "reject")}><XIcon size={15} />{t("reject")}</button>
              <button className="approve-button" disabled={busyId === approval.id} onClick={() => void onDecision(approval.id, "approve")}><CheckIcon size={15} />{t("approve")}</button>
            </div>
          </article>
        );
      })}
    </section>
  );
}
