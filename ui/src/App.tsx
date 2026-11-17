import { AnimatePresence, motion } from "motion/react";

import styles from "./App.module.css";
import { MenuHost } from "./components/Menu";
import { DownloadsView } from "./features/downloads/DownloadsView";
import { HistoryView } from "./features/history/HistoryView";
import { Onboarding } from "./features/onboarding/Onboarding";
import { ClipboardBannerView as ClipboardBanner, DropOverlay } from "./features/shell/DropOverlay";
import { Sheets } from "./features/shell/Sheets";
import { Sidebar } from "./features/shell/Sidebar";
import { Toolbar } from "./features/shell/Toolbar";
import { useGlobalInput } from "./features/shell/useGlobalInput";
import { smooth } from "./motion/tokens";
import { useApp, useConnection } from "./store/app";

export function App() {
  const ready = useApp((s) => s.ready);
  const view = useApp((s) => s.view);
  const sidebar = useApp((s) => s.settings?.sidebarVisible ?? true);
  const announcement = useApp((s) => s.announcement);
  const onboarding = useApp((s) => s.sheet?.kind === "onboarding");
  useGlobalInput();

  useConnection();

  if (!ready) return null;
  if (onboarding) return <Onboarding />;

  return (
    <div className={styles.window}>
      <AnimatePresence initial={false}>
        {sidebar && (
          <motion.nav
            key="sidebar"
            className={styles.sidebar}
            initial={{ x: "-100%" }}
            animate={{ x: 0 }}
            exit={{ x: "-100%" }}
            transition={smooth}
            aria-label="Sidebar"
          >
            <Sidebar />
          </motion.nav>
        )}
      </AnimatePresence>
      <motion.main layout="position" transition={smooth} className={styles.main} data-sidebar={sidebar}>
        <Toolbar sidebarVisible={sidebar} />
        <div className={styles.content}>
          <ClipboardBanner />
          {view === "history" ? <HistoryView /> : <DownloadsView failedOnly={view === "failed"} />}
          <DropOverlay />
        </div>
      </motion.main>
      <Sheets />
      <MenuHost />
      <div className={styles.liveRegion} aria-live="polite">
        {announcement}
      </div>
    </div>
  );
}
