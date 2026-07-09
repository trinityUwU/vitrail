import type { ReactElement } from "react";
import { SCREEN_TITLES } from "./nav-items";
import { useKillSwitch } from "../hooks/useKillSwitchState";
import { Toggle } from "../components/Toggle";
import type { ScreenId } from "../lib/types";

const STATE_LABEL: Record<string, { cls: string; text: string }> = {
  active: { cls: "on", text: "Actif" },
  transitioning: { cls: "warn", text: "Transition..." },
  degraded: { cls: "err", text: "Dégradé" },
  inactive: { cls: "off", text: "Inactif" },
};

export function Topbar({ screen }: { screen: ScreenId }): ReactElement {
  const { phase, activate, deactivate } = useKillSwitch();
  const isOn = phase === "active" || phase === "degraded";
  const stateInfo = STATE_LABEL[phase] ?? STATE_LABEL.inactive;

  const handleToggle = (): void => {
    if (isOn) void deactivate();
    else void activate();
  };

  return (
    <header id="topbar">
      <span style={{ fontSize: ".8rem", color: "var(--t3)", fontWeight: 400 }}>
        {SCREEN_TITLES[screen] ?? ""}
      </span>
      <div className="ks-quick">
        <span className="ks-quick-label">Kill Switch</span>
        <Toggle on={isOn} onToggle={handleToggle} label="Toggle kill switch" />
        <span className={`ks-quick-state ${stateInfo.cls}`}>{stateInfo.text}</span>
      </div>
    </header>
  );
}
