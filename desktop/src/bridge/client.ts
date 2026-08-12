import type {
  AttachCapabilitiesV1,
  DesktopClient,
  DesktopCommandV1,
  DesktopEventV1,
  ExternalAttachLaunchV1,
  DesktopSnapshotV1,
  DesktopUpdateV1,
  RecentWorkspaceV1,
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

  async listenRuntimeEvents(listener: (event: DesktopEventV1) => void): Promise<Unlisten> {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<DesktopEventV1>(EVENT_CHANNEL, ({ payload }) => listener(payload));
  }

  bootstrapDesktop(workspace?: string): Promise<DesktopSnapshotV1> {
    return this.invoke("bootstrap_desktop", { workspace: workspace || null });
  }

  getDesktopSnapshot(): Promise<DesktopSnapshotV1> {
    return this.invoke("get_desktop_snapshot");
  }

  dispatchDesktopCommand(command: DesktopCommandV1): Promise<void> {
    return this.invoke("dispatch_desktop_command", { command });
  }

  shutdownDesktop(): Promise<void> {
    return this.invoke("shutdown_desktop");
  }

  async listRecentWorkspaces(): Promise<RecentWorkspaceV1[]> {
    const value = await this.invoke<Array<RecentWorkspaceV1 | string>>("list_recent_workspaces");
    return value.map((item) => typeof item === "string" ? { workspace: item } : item);
  }

  forgetRecentWorkspace(workspace: string): Promise<void> {
    return this.invoke("forget_recent_workspace", { workspace });
  }

  checkDesktopUpdate(): Promise<DesktopUpdateV1> {
    return this.invoke("check_desktop_update");
  }

  getAttachCapabilities(): Promise<AttachCapabilitiesV1> {
    return this.invoke("get_attach_capabilities");
  }

  openNativeSession(member: string): Promise<ExternalAttachLaunchV1> {
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
