import type { ReactNode, ReactElement } from "react";

export function TableWrap({ children }: { children: ReactNode }): ReactElement {
  return <div className="table-wrap">{children}</div>;
}
