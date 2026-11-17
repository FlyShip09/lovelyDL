// Window chrome: the traffic lights, sheets and banners.

import { AnimatePresence, motion } from "motion/react";
import type { ReactNode } from "react";
import { useEffect, useId, useRef } from "react";

import { commands } from "../ipc";
import { fade, gentle, smooth } from "../motion/tokens";
import { s } from "../strings";
import styles from "./chrome.module.css";

const glyph = {
  close: <path d="M2 2l4 4M6 2L2 6" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />,
  minimize: <path d="M1.5 4h5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />,
  zoom: <path d="M1.8 4.6V1.8h2.8M6.2 3.4v2.8H3.4" fill="none" stroke="currentColor" strokeWidth="1.1" />,
};

/** Close, minimize and zoom, drawn as on a Mac. Keyboard users have Alt+F4 and Win+arrows. */
export function TrafficLights({
  canMinimize = true,
  canZoom = true,
}: {
  canMinimize?: boolean;
  canZoom?: boolean;
}) {
  const button = (kind: "close" | "minimize" | "zoom", label: string, enabled: boolean) => (
    <button
      type="button"
      tabIndex={-1}
      aria-label={label}
      className={`${styles.light} ${styles[kind]}`}
      disabled={!enabled}
      onClick={() => void commands.windowAction(kind)}
    >
      <svg viewBox="0 0 8 8" aria-hidden>
        {glyph[kind]}
      </svg>
    </button>
  );
  return (
    <div className={styles.lights}>
      {button("close", s.close, true)}
      {button("minimize", s.minimize, canMinimize)}
      {button("zoom", s.zoom, canZoom)}
    </div>
  );
}

const focusable = 'button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])';

interface SheetProps {
  open: boolean;
  onClose: () => void;
  label: string;
  wide?: boolean;
  children: ReactNode;
}

/** A modal sheet: slides down from under the toolbar, keeps focus inside, closes on Escape. */
export function Sheet({ open, onClose, label, wide, children }: SheetProps) {
  return (
    <AnimatePresence>
      {open && (
        <SheetBody onClose={onClose} label={label} wide={wide}>
          {children}
        </SheetBody>
      )}
    </AnimatePresence>
  );
}

function SheetBody({ onClose, label, wide, children }: Omit<SheetProps, "open">) {
  const ref = useRef<HTMLDivElement>(null);
  const id = useId();
  useEffect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const first =
      ref.current?.querySelector<HTMLElement>("[data-autofocus]") ??
      ref.current?.querySelector<HTMLElement>(focusable);
    first?.focus();
    return () => previous?.focus({ preventScroll: true });
  }, []);
  return (
    <div className={styles.sheetLayer}>
      <motion.div
        className={styles.scrim}
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={fade}
      />
      <motion.div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={label}
        id={id}
        className={`${styles.sheet} ${wide ? styles.wide : ""}`}
        initial={{ y: "-100%" }}
        animate={{ y: 0 }}
        exit={{ y: "-100%", transition: smooth }}
        transition={gentle}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.stopPropagation();
            onClose();
          } else if (e.key === "Tab" && ref.current) {
            const items = [...ref.current.querySelectorAll<HTMLElement>(focusable)];
            const [first, last] = [items[0], items[items.length - 1]];
            if (e.shiftKey && document.activeElement === first) {
              e.preventDefault();
              last?.focus();
            } else if (!e.shiftKey && document.activeElement === last) {
              e.preventDefault();
              first?.focus();
            }
          }
        }}
      >
        {children}
      </motion.div>
    </div>
  );
}

export function Banner({ open, children, label }: { open: boolean; children: ReactNode; label: string }) {
  return (
    <AnimatePresence>
      {open && (
        <motion.div
          role="status"
          aria-label={label}
          className={styles.banner}
          initial={{ y: -64, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          exit={{ y: -64, opacity: 0 }}
          transition={smooth}
        >
          {children}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
