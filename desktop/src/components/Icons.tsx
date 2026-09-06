import type { LucideIcon, LucideProps } from "lucide-react";
import {
  Activity,
  Bot,
  Check,
  ChevronRight,
  CirclePlay,
  Code2,
  Command,
  Copy,
  Cpu,
  Download,
  Ellipsis,
  FileCode2,
  Flame,
  FolderOpen,
  GitFork,
  History,
  Image,
  Languages,
  Layers,
  MessageSquareText,
  MoonStar,
  Orbit,
  PanelLeftClose,
  PanelRightClose,
  Pause,
  Play,
  Plus,
  RotateCcw,
  Search,
  Send,
  Settings2,
  Shield,
  ShieldCheck,
  Sparkles,
  Square,
  SunMedium,
  Terminal,
  TriangleAlert,
  UsersRound,
  Wrench,
  X,
  Zap,
} from "lucide-react";

export type IconProps = LucideProps;

function withDefaults(Component: LucideIcon) {
  return function AsterlineIcon({ className, size = 20, strokeWidth = 1.8, ...props }: IconProps) {
    return (
      <Component
        aria-hidden="true"
        focusable="false"
        absoluteStrokeWidth
        className={["ui-icon", className].filter(Boolean).join(" ")}
        size={size}
        strokeWidth={strokeWidth}
        {...props}
      />
    );
  };
}

export const PlusIcon = withDefaults(Plus);
export const FolderIcon = withDefaults(FolderOpen);
export const ChatIcon = withDefaults(MessageSquareText);
export const SettingsIcon = withDefaults(Settings2);
export const ApprovalIcon = withDefaults(ShieldCheck);
export const ModeIcon = withDefaults(Orbit);
export const SunIcon = withDefaults(SunMedium);
export const MoonIcon = withDefaults(MoonStar);
export const GlobeIcon = withDefaults(Languages);
export const PanelLeftIcon = withDefaults(PanelLeftClose);
export const PanelRightIcon = withDefaults(PanelRightClose);
export const SendIcon = withDefaults(Send);
export const StopIcon = withDefaults(Square);
export const ChevronIcon = withDefaults(ChevronRight);
export const CheckIcon = withDefaults(Check);
export const XIcon = withDefaults(X);
export const UsersIcon = withDefaults(UsersRound);
export const RunIcon = withDefaults(CirclePlay);
export const ToolIcon = withDefaults(Wrench);
export const RouteIcon = withDefaults(GitFork);
export const FileIcon = withDefaults(FileCode2);
export const AlertIcon = withDefaults(TriangleAlert);
export const SparkIcon = withDefaults(Sparkles);
export const MoreIcon = withDefaults(Ellipsis);
export const HistoryIcon = withDefaults(History);
export const ImageIcon = withDefaults(Image);
export const PauseIcon = withDefaults(Pause);
export const PlayIcon = withDefaults(Play);
export const RefreshIcon = withDefaults(RotateCcw);
export const SearchIcon = withDefaults(Search);
export const TerminalIcon = withDefaults(Terminal);
export const ActivityIcon = withDefaults(Activity);
export const BotIcon = withDefaults(Bot);
export const CodeIcon = withDefaults(Code2);
export const CommandIcon = withDefaults(Command);
export const CopyIcon = withDefaults(Copy);
export const DownloadIcon = withDefaults(Download);
export const CpuIcon = withDefaults(Cpu);
export const FlameIcon = withDefaults(Flame);
export const LayersIcon = withDefaults(Layers);
export const ShieldIcon = withDefaults(Shield);
export const ZapIcon = withDefaults(Zap);
