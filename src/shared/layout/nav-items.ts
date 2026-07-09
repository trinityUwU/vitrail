import type { LucideIcon } from "lucide-react";
import {
  Activity,
  AppWindow,
  Bell,
  Clock,
  FileText,
  Globe,
  LayoutDashboard,
  Lock,
  Search,
  Settings as SettingsIcon,
  ShieldCheck,
} from "lucide-react";
import type { ScreenId } from "../lib/types";

export interface NavItem {
  id: ScreenId;
  icon: LucideIcon;
  label: string;
}

export interface NavGroup {
  group: string;
  items: NavItem[];
}

export const NAV_GROUPS: NavGroup[] = [
  {
    group: "Surveillance",
    items: [
      { id: "dashboard", icon: LayoutDashboard, label: "Vue d'ensemble" },
      { id: "timeline", icon: Activity, label: "Timeline" },
      { id: "processes", icon: AppWindow, label: "Processus" },
      { id: "destinations", icon: Globe, label: "Destinations" },
    ],
  },
  {
    group: "Analyse",
    items: [
      { id: "search", icon: Search, label: "Recherche avancée" },
      { id: "history", icon: Clock, label: "Historique" },
    ],
  },
  {
    group: "Sécurité",
    items: [
      { id: "killswitch", icon: ShieldCheck, label: "Kill Switch" },
      { id: "alerts", icon: Bell, label: "Alertes" },
    ],
  },
  {
    group: "Système",
    items: [
      { id: "settings", icon: SettingsIcon, label: "Paramètres" },
      { id: "logs", icon: FileText, label: "Journal système" },
      { id: "privacy", icon: Lock, label: "Confidentialité" },
    ],
  },
];

export const SCREEN_TITLES: Record<ScreenId, string> = {
  onboarding: "Configuration initiale",
  dashboard: "Vue d'ensemble",
  timeline: "Timeline temps réel",
  processes: "Processus",
  destinations: "Destinations",
  inspector: "Inspecteur de flux",
  search: "Recherche avancée",
  alerts: "Alertes & Règles",
  killswitch: "Kill Switch",
  settings: "Paramètres",
  logs: "Journal système",
  privacy: "Confidentialité",
  history: "Historique",
};
