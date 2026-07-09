import type { ButtonHTMLAttributes, ReactNode, ReactElement } from "react";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "primary" | "danger" | "ghost";
  size?: "md" | "sm" | "lg";
  children: ReactNode;
}

export function Button({
  variant = "default",
  size = "md",
  className = "",
  children,
  ...rest
}: ButtonProps): ReactElement {
  const variantClass = variant === "default" ? "" : ` btn-${variant}`;
  const sizeClass = size === "md" ? "" : ` btn-${size}`;
  return (
    <button className={`btn${variantClass}${sizeClass} ${className}`.trim()} {...rest}>
      {children}
    </button>
  );
}
