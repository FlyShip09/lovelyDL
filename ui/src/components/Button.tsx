import type { LucideIcon } from "lucide-react";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import { forwardRef } from "react";

import styles from "./Button.module.css";
import { Icon } from "./Icon";

type Variant = "default" | "primary" | "plain" | "destructive";

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  large?: boolean;
  children: ReactNode;
}

export function Button({ variant = "default", large, className, type = "button", ...rest }: Props) {
  const classes = [
    styles.button,
    large && styles.large,
    variant === "primary" && styles.primary,
    variant === "plain" && styles.plain,
    variant === "destructive" && styles.destructive,
    className,
  ];
  return <button type={type} className={classes.filter(Boolean).join(" ")} {...rest} />;
}

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: LucideIcon;
  label: string;
  small?: boolean;
}

/** A borderless square button with only an icon. The label is its accessible name and tooltip. */
export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { icon, label, small, className, type = "button", ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      aria-label={label}
      title={label}
      className={[styles.iconButton, small && styles.small, className].filter(Boolean).join(" ")}
      {...rest}
    >
      <Icon icon={icon} size={small ? 14 : 16} />
    </button>
  );
});
