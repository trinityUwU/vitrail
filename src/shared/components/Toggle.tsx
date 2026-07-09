import type { ReactElement } from "react";
interface ToggleProps {
  on: boolean;
  onToggle: () => void;
  label: string;
  size?: "md" | "lg";
}

export function Toggle({ on, onToggle, label, size = "md" }: ToggleProps): ReactElement {
  const className = size === "lg" ? "ks-big-toggle" : "toggle";
  return (
    <button
      type="button"
      className={`${className}${on ? " on" : ""}`}
      onClick={onToggle}
      aria-label={label}
      aria-pressed={on}
    />
  );
}
