import type { ReactElement } from "react";
import { NAV_GROUPS } from "./nav-items";
import { useKillSwitch } from "../hooks/useKillSwitchState";
import { useAlertBadge } from "../hooks/useAlertBadge";
import type { ScreenId } from "../lib/types";

interface SidebarProps {
  activeScreen: ScreenId;
  onNavigate: (screen: ScreenId) => void;
}

export function Sidebar({ activeScreen, onNavigate }: SidebarProps): ReactElement {
  const { phase } = useKillSwitch();
  const alertBadge = useAlertBadge();

  return (
    <aside id="sidebar">
      <div className="sidebar-brand">
        <span className={`sidebar-brand-dot ${phase}`} />
        <h1>Vitrail</h1>
      </div>
      <nav className="sidebar-nav">
        {NAV_GROUPS.map((group) => (
          <div className="nav-group" key={group.group}>
            <div className="nav-group-label">{group.group}</div>
            {group.items.map((item) => {
              const Icon = item.icon;
              const badge = item.id === "alerts" && alertBadge > 0 ? alertBadge : null;
              return (
                <div
                  key={item.id}
                  className={`nav-item${activeScreen === item.id ? " active" : ""}`}
                  onClick={() => onNavigate(item.id)}
                >
                  <Icon />
                  <span>{item.label}</span>
                  {badge !== null && <span className="nav-badge">{badge}</span>}
                </div>
              );
            })}
          </div>
        ))}
      </nav>
    </aside>
  );
}
