import type { ReactNode, ReactElement } from "react";
import { KillSwitchContext, useKillSwitchProviderState } from "../hooks/useKillSwitchState";

export function KillSwitchProvider({ children }: { children: ReactNode }): ReactElement {
  const value = useKillSwitchProviderState();
  return <KillSwitchContext.Provider value={value}>{children}</KillSwitchContext.Provider>;
}
