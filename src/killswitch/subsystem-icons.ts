import { Eye, Download, KeyRound, FileKey, Shield, BadgeCheck, type LucideIcon } from "lucide-react";

export const SUBSYSTEM_ICONS: Record<string, LucideIcon> = {
  opensnitch: Eye,
  capture: Download,
  polarproxy: KeyRound,
  keylog: FileKey,
  nftables: Shield,
  ca: BadgeCheck,
};
