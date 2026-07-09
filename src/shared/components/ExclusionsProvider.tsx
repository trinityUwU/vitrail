import type { ReactNode, ReactElement } from "react";
import { ExclusionsContext, useExclusionsProviderState } from "../hooks/useExclusionsState";

export function ExclusionsProvider({ children }: { children: ReactNode }): ReactElement {
  const value = useExclusionsProviderState();
  return <ExclusionsContext.Provider value={value}>{children}</ExclusionsContext.Provider>;
}
