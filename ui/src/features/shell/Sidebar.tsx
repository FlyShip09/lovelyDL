import type { LucideIcon } from "lucide-react";
import { ArrowDownToLine, Clock, PanelLeft, TriangleAlert } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";

import { IconButton } from "../../components/Button";
import { TrafficLights } from "../../components/chrome";
import { Icon } from "../../components/Icon";
import { snappy } from "../../motion/tokens";
import { setView, useApp, type View } from "../../store/app";
import { s } from "../../strings";
import styles from "./shell.module.css";
import { toggleSidebar } from "./Toolbar";

function useCounts() {
  return useApp((st) => {
    let downloads = 0;
    let failed = 0;
    for (const job of Object.values(st.jobs)) {
      if (job.cleared) continue;
      if (job.state === "failed") failed += 1;
      else if (job.state !== "completed" && job.state !== "canceled") downloads += 1;
    }
    return `${downloads}:${failed}`;
  });
}

/** A count that pulses when it changes, and never at launch. */
function Count({ value }: { value: number }) {
  return (
    <AnimatePresence initial={false} mode="popLayout">
      <motion.span
        key={value}
        className={`${styles.count} tabular`}
        initial={{ scale: 1.3 }}
        animate={{ scale: 1 }}
        exit={{ opacity: 0, transition: { duration: 0 } }}
        transition={snappy}
      >
        {value}
      </motion.span>
    </AnimatePresence>
  );
}

export function Sidebar() {
  const view = useApp((st) => st.view);
  const [downloads, failed] = useCounts().split(":").map(Number);
  const items: { id: View; label: string; icon: LucideIcon; count?: number }[] = [
    { id: "downloads", label: s.downloads, icon: ArrowDownToLine, count: downloads },
    { id: "history", label: s.history, icon: Clock },
    { id: "failed", label: s.failed, icon: TriangleAlert, count: failed },
  ];
  return (
    <>
      <div className={styles.sidebarTop} data-tauri-drag-region>
        <TrafficLights />
        <IconButton icon={PanelLeft} label={s.toggleSidebar} onClick={toggleSidebar} />
      </div>
      <div className={styles.list} role="tablist" aria-orientation="vertical" aria-label={s.sections}>
        {items.map((item, index) => (
          <button
            key={item.id}
            type="button"
            role="tab"
            aria-selected={view === item.id}
            tabIndex={view === item.id ? 0 : -1}
            className={styles.item}
            onClick={() => setView(item.id)}
            onKeyDown={(e) => {
              const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
              const next = items[(index + step + items.length) % items.length];
              if (step && next) {
                e.preventDefault();
                setView(next.id);
                const sibling = e.currentTarget.parentElement?.children[items.indexOf(next)];
                if (sibling instanceof HTMLElement) sibling.focus();
              }
            }}
          >
            {view === item.id && (
              <motion.span layoutId="sidebar-highlight" className={styles.highlight} transition={snappy} />
            )}
            <span className={styles.itemIcon}>
              <Icon icon={item.icon} size={16} />
            </span>
            <span className={styles.itemLabel}>{item.label}</span>
            {item.count ? <Count value={item.count} /> : null}
          </button>
        ))}
      </div>
    </>
  );
}
