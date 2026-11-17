// Small controls, each built to look and behave like its macOS counterpart.

import { Check, ChevronsUpDown, Minus, Search } from "lucide-react";
import { animate, motion, useMotionValue } from "motion/react";
import type { InputHTMLAttributes, ReactNode } from "react";
import { forwardRef, useEffect, useId, useRef } from "react";

import { snappy } from "../motion/tokens";
import styles from "./controls.module.css";
import { Icon } from "./Icon";
import { type MenuItem, openMenuBelow } from "./Menu";

export interface Choice<T> {
  value: T;
  label: string;
  disabled?: boolean;
}

interface PopUpProps<T> {
  value: T;
  choices: (Choice<T> | "separator")[];
  onChange: (value: T) => void;
  label: string;
  display?: string;
  disabled?: boolean;
}

/** A button that shows the current choice and opens a menu of the others. */
export function PopUpButton<T>({ value, choices, onChange, label, display, disabled }: PopUpProps<T>) {
  const ref = useRef<HTMLButtonElement>(null);
  const current = choices.find((c): c is Choice<T> => c !== "separator" && c.value === value);
  const open = () => {
    if (!ref.current) return;
    const items: MenuItem[] = choices.map((c) =>
      c === "separator"
        ? { separator: true }
        : {
            label: c.label,
            checked: c.value === value,
            disabled: c.disabled,
            onSelect: () => onChange(c.value),
          },
    );
    openMenuBelow(ref.current, items);
  };
  return (
    <button
      ref={ref}
      type="button"
      className={styles.popup}
      aria-haspopup="menu"
      aria-label={`${label}: ${display ?? current?.label ?? ""}`}
      disabled={disabled}
      onClick={open}
      onKeyDown={(e) => {
        if (e.key === "ArrowDown" || e.key === "ArrowUp") {
          e.preventDefault();
          open();
        }
      }}
    >
      <span className={styles.popupLabel}>{display ?? current?.label}</span>
      <span className={styles.chevrons}>
        <Icon icon={ChevronsUpDown} size={11} />
      </span>
    </button>
  );
}

interface SegmentedProps<T> {
  value: T;
  choices: Choice<T>[];
  onChange: (value: T) => void;
  label: string;
}

/** The selected segment is a raised pill that slides to the new choice. */
export function SegmentedControl<T extends string>({ value, choices, onChange, label }: SegmentedProps<T>) {
  const id = useId();
  return (
    <div role="radiogroup" aria-label={label} className={styles.segmented}>
      {choices.map((choice, index) => (
        <button
          key={choice.value}
          type="button"
          role="radio"
          aria-checked={choice.value === value}
          tabIndex={choice.value === value ? 0 : -1}
          className={styles.segment}
          disabled={choice.disabled}
          onClick={() => onChange(choice.value)}
          onKeyDown={(e) => {
            const step = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
            const next = choices[(index + step + choices.length) % choices.length];
            if (step && next) {
              e.preventDefault();
              onChange(next.value);
              const sibling = e.currentTarget.parentElement?.children[choices.indexOf(next)];
              if (sibling instanceof HTMLElement) sibling.focus();
            }
          }}
        >
          {choice.value === value && (
            <motion.span layoutId={`seg-${id}`} className={styles.segmentPill} transition={snappy} />
          )}
          {choice.label}
        </button>
      ))}
    </div>
  );
}

interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  disabled?: boolean;
}

export function Toggle({ checked, onChange, label, disabled }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      className={styles.toggle}
      onClick={() => onChange(!checked)}
    >
      <motion.span
        className={styles.knob}
        animate={{ x: checked ? 14 : 0 }}
        transition={snappy}
        initial={false}
      />
    </button>
  );
}

interface CheckboxProps {
  checked: boolean | "mixed";
  onChange: (checked: boolean) => void;
  label: ReactNode;
  disabled?: boolean;
  /** Hide the visible label; `ariaLabel` still names the checkbox. */
  hideLabel?: boolean;
  ariaLabel?: string;
}

export function Checkbox({ checked, onChange, label, disabled, hideLabel, ariaLabel }: CheckboxProps) {
  const id = useId();
  return (
    <span className={styles.checkbox}>
      <button
        type="button"
        role="checkbox"
        aria-checked={checked}
        aria-labelledby={hideLabel ? undefined : id}
        aria-label={hideLabel ? ariaLabel : undefined}
        disabled={disabled}
        className={styles.box}
        onClick={(e) => {
          e.stopPropagation();
          onChange(checked !== true);
        }}
      >
        {checked === true && <Icon icon={Check} size={11} />}
        {checked === "mixed" && <Icon icon={Minus} size={11} />}
      </button>
      {!hideLabel && (
        <span
          id={id}
          onClick={() => {
            if (!disabled) onChange(checked !== true);
          }}
        >
          {label}
        </span>
      )}
    </span>
  );
}

export const TextField = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function TextField({ className, ...rest }, ref) {
    return (
      <input
        ref={ref}
        spellCheck={false}
        className={[styles.field, className].filter(Boolean).join(" ")}
        {...rest}
      />
    );
  },
);

interface SearchProps extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
}

export const SearchField = forwardRef<HTMLInputElement, SearchProps>(function SearchField(
  { label, className, ...rest },
  ref,
) {
  return (
    <div className={[styles.search, className].filter(Boolean).join(" ")}>
      <span className={styles.searchIcon}>
        <Icon icon={Search} size={13} />
      </span>
      <input
        ref={ref}
        type="search"
        aria-label={label}
        placeholder={label}
        spellCheck={false}
        className={styles.field}
        {...rest}
      />
    </div>
  );
});

interface ProgressProps {
  /** 0 to 1, or null when the amount is unknown. */
  value: number | null;
  /** Changes when a new stage starts; only then may the bar move backwards. */
  stage?: string;
  label: string;
}

/**
 * A thin bar that glides between updates instead of jumping, and never moves backwards
 * except when a new stage starts.
 */
export function ProgressBar({ value, stage, label }: ProgressProps) {
  const scale = useMotionValue(value ?? 0);
  const last = useRef({ value: value ?? 0, stage });
  useEffect(() => {
    if (value === null) return;
    const restart = stage !== last.current.stage;
    const target = restart ? value : Math.max(value, last.current.value);
    last.current = { value: target, stage };
    if (restart) scale.set(target);
    // About as long as the gap between two updates, so the bar never stops in between.
    else void animate(scale, target, { duration: 0.3, ease: "linear" });
  }, [value, stage, scale]);
  return (
    <div
      className={styles.track}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={value === null ? undefined : Math.round(value * 100)}
    >
      {value === null ? (
        <span className={styles.indeterminate} />
      ) : (
        <motion.span className={styles.bar} style={{ scaleX: scale }} />
      )}
    </div>
  );
}

export function Spinner({ size = 14, label }: { size?: number; label?: string }) {
  return (
    <span
      role={label ? "status" : undefined}
      aria-label={label}
      className={styles.spinner}
      style={{ width: size, height: size }}
    />
  );
}

export type { MenuItem };
