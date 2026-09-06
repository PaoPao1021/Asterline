import type {
  AttachCapabilities,
  BackendAvailability,
  BackendKind,
  CommandSpec,
  ComposerAction,
  Completion,
  DesktopClient,
  DesktopCommandV2,
  DiagnosticsStatus,
  DesktopEventV2,
  ExternalAttachLaunch,
  DesktopLaunchOptions,
  DesktopSnapshotV2,
  DesktopUpdate,
  ModelSummary,
  NativeSessionSummary,
  RecentWorkspace,
  StagedAttachment,
  Unlisten,
} from "./types";
import { createMockDesktopClient } from "./mock";

const EVENT_CHANNEL = "asterline://runtime-event";

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

class TauriDesktopClient implements DesktopClient {
  readonly kind = "tauri" as const;

  private async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(command, args);
  }

  async listenRuntimeEvents(listener: (event: DesktopEventV2) => void): Promise<Unlisten> {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<DesktopEventV2>(EVENT_CHANNEL, ({ payload }) => listener(payload));
  }

  bootstrapDesktop(workspace?: string, options?: DesktopLaunchOptions): Promise<DesktopSnapshotV2> {
    return this.invoke("bootstrap_desktop", {
      workspace: workspace || null,
      options: options ?? null,
    });
  }

  getDesktopSnapshot(): Promise<DesktopSnapshotV2> {
    return this.invoke("get_desktop_snapshot");
  }

  parseComposerText(text: string): Promise<ComposerAction> {
    return this.invoke("parse_composer_text", { text });
  }

  completeComposer(head: string): Promise<Completion | null> {
    return this.invoke("complete_composer", { head });
  }

  commandCatalog(): Promise<CommandSpec[]> {
    return this.invoke("command_catalog");
  }

  getBackendAvailability(): Promise<BackendAvailability> {
    return this.invoke("get_backend_availability");
  }

  listModels(backend: BackendKind, cwd: string): Promise<ModelSummary[]> {
    return this.invoke("list_models", { backend, cwd });
  }

  listNativeSessions(backend: BackendKind, cwd: string): Promise<NativeSessionSummary[]> {
    return this.invoke("list_native_sessions", { backend, cwd });
  }

  stageClipboardImage(): Promise<StagedAttachment> {
    return this.invoke("stage_clipboard_image");
  }

  stageImagePath(path: string): Promise<StagedAttachment> {
    return this.invoke("stage_image_path", { path });
  }

  stageImageBytes(base64: string): Promise<StagedAttachment> {
    return this.invoke("stage_image_bytes", { bytesBase64: base64 });
  }

  removeStagedAttachment(token: string): Promise<void> {
    return this.invoke("remove_staged_attachment", { token });
  }

  discardStagedAttachments(): Promise<void> {
    return this.invoke("discard_staged_attachments");
  }

  dispatchDesktopCommand(command: DesktopCommandV2): Promise<void> {
    return this.invoke("dispatch_desktop_command", { command });
  }

  shutdownDesktop(): Promise<void> {
    return this.invoke("shutdown_desktop");
  }

  exitDesktop(): Promise<void> {
    return this.invoke("exit_desktop");
  }

  async listRecentWorkspaces(): Promise<RecentWorkspace[]> {
    const value = await this.invoke<Array<RecentWorkspace | string>>("list_recent_workspaces");
    return value.map((item) => typeof item === "string" ? { workspace: item } : item);
  }

  forgetRecentWorkspace(workspace: string): Promise<void> {
    return this.invoke("forget_recent_workspace", { workspace });
  }

  checkDesktopUpdate(): Promise<DesktopUpdate> {
    return this.invoke("check_desktop_update");
  }

  openDesktopUpdate(url: string): Promise<void> {
    return this.invoke("open_desktop_update", { url });
  }

  getDesktopDiagnosticsStatus(): Promise<DiagnosticsStatus> {
    return this.invoke("get_desktop_diagnostics_status");
  }

  exportDesktopDiagnostics(): Promise<string> {
    return this.invoke("export_desktop_diagnostics");
  }

  getAttachCapabilities(): Promise<AttachCapabilities> {
    return this.invoke("get_attach_capabilities");
  }

  openNativeSession(member: string): Promise<ExternalAttachLaunch> {
    return this.invoke("open_native_session", { member });
  }
}

let sharedClient: DesktopClient | undefined;

export function getDesktopClient(): DesktopClient {
  if (!sharedClient) {
    sharedClient = isTauriRuntime() ? new TauriDesktopClient() : createMockDesktopClient();
  }
  return sharedClient;
}

export function setDesktopClientForTests(client?: DesktopClient): void {
  sharedClient = client;
}

/** Bytes → base64 without pulling in a dependency (file/clipboard staging). */
export function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += chunk) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + chunk));
  }
  return btoa(binary);
}
