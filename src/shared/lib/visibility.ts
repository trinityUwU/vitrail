import type { FlowVisibility } from "./types";

interface VisibilityMeta {
  label: string;
  badgeClass: string;
}

const VISIBILITY_MAP: Record<FlowVisibility, VisibilityMeta> = {
  fully: { label: "Déchiffré", badgeClass: "badge-ok" },
  meta: { label: "Métadonnées", badgeClass: "badge-meta" },
  attrib: { label: "Attribué", badgeClass: "badge-attrib" },
  unknown: { label: "Inconnu", badgeClass: "badge-unknown" },
};

export function visibilityMeta(visibility: FlowVisibility): VisibilityMeta {
  return VISIBILITY_MAP[visibility] ?? VISIBILITY_MAP.unknown;
}

export const VISIBILITY_OPTIONS: Array<{ key: FlowVisibility; label: string }> = (
  Object.keys(VISIBILITY_MAP) as FlowVisibility[]
).map((key) => ({ key, label: VISIBILITY_MAP[key].label }));
