// Menus: pop-up buttons, the toolbar's More menu and every context menu. One host renders
// whichever menu is open, so there is never more than one.

import { Check } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { create } from "zustand";

import { fade } from "../motion/tokens";
import { Icon } from "./Icon";
import styles from "./Menu.module.css";

export type MenuItem =
  | {
      label: string;
      onSelect?: () => void;
      disabled?: boolean;
      checked?: boolean;
      destructive?: boolean;
      shortcut?: string;
    }
  | { separator: true };

type Anchor = { x: number; y: number } | { rect: DOMRect; align?: "start" | "end" };

interface MenuState {
  items: MenuItem[] | null;
  anchor: Anchor | null;
  minWidth: number;
  returnFocus: HTMLElement | null;
  key: number;
}

const useMenu = create<MenuState>()(() => ({
  items: null,
  anchor: null,
  minWidth: 0,
  returnFocus: null,
  key: 0,
}));

export function openMenu(items: MenuItem[], anchor: Anchor, options: { minWidth?: number } = {}) {
  useMenu.setState((s) => ({
    items,
    anchor,
    minWidth: options.minWidth ?? 0,
    returnFocus: document.activeElement instanceof HTMLElement ? document.activeElement : null,
    key: s.key + 1,
  }));
}

export function closeMenu() {
  const { returnFocus } = useMenu.getState();
  useMenu.setState({ items: null });
  returnFocus?.focus({ preventScroll: true });
}

/** Opens a menu below an element, as a pop-up button or a toolbar button does. */
export function openMenuBelow(element: HTMLElement, items: MenuItem[], align: "start" | "end" = "start") {
  openMenu(items, { rect: element.getBoundingClientRect(), align }, { minWidth: element.offsetWidth });
}

function isAction(item: MenuItem): item is Exclude<MenuItem, { separator: true }> {
  return !("separator" in item);
}

export function MenuHost() {
  const { items, anchor, minWidth, key } = useMenu();
  return (
    <AnimatePresence>
      {items && anchor && <MenuPanel key={key} items={items} anchor={anchor} minWidth={minWidth} />}
    </AnimatePresence>
  );
}

function MenuPanel({ items, anchor, minWidth }: { items: MenuItem[]; anchor: Anchor; minWidth: number }) {
  const ref = useRef<HTMLDivElement>(null);
  const [active, setActive] = useState(-1);
  const [place, setPlace] = useState<{ left: number; top: number; origin: string } | null>(null);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    const margin = 6;
    let left: number;
    let top: number;
    let origin: string;
    if ("rect" in anchor) {
      left = anchor.align === "end" ? anchor.rect.right - width : anchor.rect.left;
      top = anchor.rect.bottom + 4;
      if (top + height > innerHeight - margin) top = anchor.rect.top - height - 4;
      origin = `${anchor.align === "end" ? "right" : "left"} top`;
    } else {
      left = anchor.x;
      top = anchor.y;
      if (left + width > innerWidth - margin) left = anchor.x - width;
      if (top + height > innerHeight - margin) top = Math.max(margin, anchor.y - height);
      origin = "left top";
    }
    left = Math.min(Math.max(margin, left), innerWidth - width - margin);
    setPlace({ left, top, origin });
    el.focus({ preventScroll: true });
  }, [anchor]);

  const choose = (index: number) => {
    const item = items[index];
    if (!item || !isAction(item) || item.disabled) return;
    closeMenu();
    item.onSelect?.();
  };

  const move = (delta: number) => {
    const count = items.length;
    let next = active;
    for (let i = 0; i < count; i++) {
      next = (next + delta + count) % count;
      const item = items[next];
      if (item && isAction(item) && !item.disabled) break;
    }
    setActive(next);
  };

  useEffect(() => {
    const onBlur = () => closeMenu();
    window.addEventListener("blur", onBlur);
    window.addEventListener("resize", onBlur);
    return () => {
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("resize", onBlur);
    };
  }, []);

  return (
    <div
      className={styles.layer}
      onPointerDown={(e) => e.target === e.currentTarget && closeMenu()}
      onContextMenu={(e) => {
        e.preventDefault();
        closeMenu();
      }}
    >
      <motion.div
        ref={ref}
        role="menu"
        tabIndex={-1}
        className={styles.menu}
        style={{
          minWidth: Math.max(180, minWidth),
          left: place?.left ?? -9999,
          top: place?.top ?? -9999,
          transformOrigin: place?.origin,
        }}
        initial={{ opacity: 0, scale: 0.97 }}
        animate={{ opacity: 1, scale: 1 }}
        exit={{ opacity: 0, transition: { duration: 0.1 } }}
        transition={fade}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown") move(1);
          else if (e.key === "ArrowUp") move(-1);
          else if (e.key === "Enter" || e.key === " ") choose(active);
          else if (e.key === "Escape" || e.key === "Tab") closeMenu();
          else return;
          e.preventDefault();
          e.stopPropagation();
        }}
      >
        {items.map((item, index) =>
          isAction(item) ? (
            <button
              key={index}
              type="button"
              role={item.checked === undefined ? "menuitem" : "menuitemcheckbox"}
              aria-checked={item.checked}
              disabled={item.disabled}
              tabIndex={-1}
              className={[styles.item, item.destructive && styles.destructive].filter(Boolean).join(" ")}
              data-active={index === active}
              onPointerEnter={() => !item.disabled && setActive(index)}
              onPointerLeave={() => setActive(-1)}
              onClick={() => choose(index)}
            >
              <span className={styles.check}>{item.checked && <Icon icon={Check} size={12} />}</span>
              <span className={styles.label}>{item.label}</span>
              {item.shortcut && <span className={styles.shortcut}>{item.shortcut}</span>}
            </button>
          ) : (
            <div key={index} role="separator" className={styles.separator} />
          ),
        )}
      </motion.div>
    </div>
  );
}
