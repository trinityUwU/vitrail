import type { ReactElement } from "react";
import { Badge } from "./Badge";
import { visibilityMeta } from "../lib/visibility";
import type { FlowVisibility } from "../lib/types";

const VARIANT_BY_CLASS: Record<string, "ok" | "meta" | "attrib" | "unknown"> = {
  "badge-ok": "ok",
  "badge-meta": "meta",
  "badge-attrib": "attrib",
  "badge-unknown": "unknown",
};

export function VisibilityBadge({ visibility }: { visibility: FlowVisibility }): ReactElement {
  const meta = visibilityMeta(visibility);
  const variant = VARIANT_BY_CLASS[meta.badgeClass] ?? "unknown";
  return (
    <Badge variant={variant} withDot>
      {meta.label}
    </Badge>
  );
}
