import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement> & { size?: number };

function Icon({ size = 20, children, ...props }: IconProps) {
  const { className, ...svgProps } = props;
  return <svg className={["ui-icon", className].filter(Boolean).join(" ")} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" shapeRendering="geometricPrecision" focusable="false" aria-hidden="true" {...svgProps}>{children}</svg>;
}

export const PlusIcon = (p: IconProps) => <Icon {...p}><path d="M12 5v14M5 12h14" /></Icon>;
export const FolderIcon = (p: IconProps) => <Icon {...p}><path d="M3.5 6.5h6l2 2h9v9a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z" /></Icon>;
export const ChatIcon = (p: IconProps) => <Icon {...p}><path d="M20 15a3 3 0 0 1-3 3H9l-5 3v-6a3 3 0 0 1-1-2.2V7a3 3 0 0 1 3-3h11a3 3 0 0 1 3 3z" /></Icon>;
export const SettingsIcon = (p: IconProps) => <Icon {...p}><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4V21a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9A1.7 1.7 0 0 0 3 14H2.8v-4H3a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-1.6v-.2h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1z" /></Icon>;
export const SunIcon = (p: IconProps) => <Icon {...p}><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></Icon>;
export const MoonIcon = (p: IconProps) => <Icon {...p}><path d="M20 15.5A8.5 8.5 0 0 1 8.5 4 8.5 8.5 0 1 0 20 15.5z"/></Icon>;
export const GlobeIcon = (p: IconProps) => <Icon {...p}><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a15 15 0 0 1 0 18M12 3a15 15 0 0 0 0 18"/></Icon>;
export const PanelLeftIcon = (p: IconProps) => <Icon {...p}><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/></Icon>;
export const PanelRightIcon = (p: IconProps) => <Icon {...p}><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/></Icon>;
export const SendIcon = (p: IconProps) => <Icon {...p}><path d="m22 2-7 20-4-9-9-4zM22 2 11 13"/></Icon>;
export const StopIcon = (p: IconProps) => <Icon {...p}><rect x="6" y="6" width="12" height="12" rx="2"/></Icon>;
export const ChevronIcon = (p: IconProps) => <Icon {...p}><path d="m9 18 6-6-6-6"/></Icon>;
export const CheckIcon = (p: IconProps) => <Icon {...p}><path d="m5 12 4 4L19 6"/></Icon>;
export const XIcon = (p: IconProps) => <Icon {...p}><path d="M18 6 6 18M6 6l12 12"/></Icon>;
export const UsersIcon = (p: IconProps) => <Icon {...p}><path d="M16 20v-1.5a3.5 3.5 0 0 0-3.5-3.5h-5A3.5 3.5 0 0 0 4 18.5V20M10 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8M17 11a3 3 0 0 0 0-6M20 20v-1.5a3.5 3.5 0 0 0-2.5-3.4"/></Icon>;
export const RunIcon = (p: IconProps) => <Icon {...p}><circle cx="12" cy="12" r="9"/><path d="m10 8 6 4-6 4z"/></Icon>;
export const ToolIcon = (p: IconProps) => <Icon {...p}><path d="M14.7 6.3a4 4 0 0 0-5-5L12 3.6 8.6 7 6.3 4.7a4 4 0 0 0 5 5l7.6 7.6a2 2 0 0 1-2.8 2.8l-7.6-7.6"/></Icon>;
export const RouteIcon = (p: IconProps) => <Icon {...p}><circle cx="6" cy="6" r="2"/><circle cx="18" cy="18" r="2"/><path d="M8 6h4a4 4 0 0 1 4 4v6M12 10l4 4 4-4"/></Icon>;
export const FileIcon = (p: IconProps) => <Icon {...p}><path d="M6 2h8l4 4v16H6zM14 2v5h5"/></Icon>;
export const AlertIcon = (p: IconProps) => <Icon {...p}><path d="M12 3 2.8 20h18.4zM12 9v4M12 17h.01"/></Icon>;
export const SparkIcon = (p: IconProps) => <Icon {...p}><path d="m12 3 1.4 4.6L18 9l-4.6 1.4L12 15l-1.4-4.6L6 9l4.6-1.4zM19 15l.7 2.3L22 18l-2.3.7L19 21l-.7-2.3L16 18l2.3-.7z"/></Icon>;
export const MoreIcon = (p: IconProps) => <Icon {...p}><circle cx="5" cy="12" r="1" fill="currentColor"/><circle cx="12" cy="12" r="1" fill="currentColor"/><circle cx="19" cy="12" r="1" fill="currentColor"/></Icon>;
export const HistoryIcon = (p: IconProps) => <Icon {...p}><path d="M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5M12 7v5l3 2"/></Icon>;
export const RefreshIcon = (p: IconProps) => <Icon {...p}><path d="M20 7v5h-5M4 17v-5h5M6.1 8A7 7 0 0 1 18.7 6L20 8M4 16l1.3 2A7 7 0 0 0 18 16"/></Icon>;
