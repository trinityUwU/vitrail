import type { ReactNode, ReactElement } from "react";
import { ToastContext, useToastState } from "../hooks/useToast";

export function ToastProvider({ children }: { children: ReactNode }): ReactElement {
  const value = useToastState();
  return (
    <ToastContext.Provider value={value}>
      {children}
      <div id="toast-container">
        {value.toasts.map((t) => (
          <div key={t.id} className="toast">
            {t.message}
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}
