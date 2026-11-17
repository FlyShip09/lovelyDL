import { AnimatePresence, motion } from "motion/react";

import { Button } from "../../components/Button";
import { Banner } from "../../components/chrome";
import { fade } from "../../motion/tokens";
import { acceptClipboard, dismissClipboard, useApp } from "../../store/app";
import { s } from "../../strings";
import styles from "./shell.module.css";
import { useDragging } from "./useGlobalInput";

/** While a link is dragged over the window: the content dims and an accent outline appears. */
export function DropOverlay() {
  const over = useDragging((st) => st.over);
  return (
    <AnimatePresence>
      {over && (
        <motion.div
          className={styles.drop}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={fade}
        >
          <span className={styles.dropHint}>{s.dropHint}</span>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

/** Offers a link that was copied while lovelyDL was in the background. */
export function ClipboardBannerView() {
  const open = useApp((st) => st.clipboardOffer);
  return (
    <Banner open={open} label={s.clipboardOffer}>
      <span className={styles.bannerText}>{s.clipboardOffer}</span>
      <Button variant="plain" onClick={dismissClipboard}>
        {s.clipboardDismiss}
      </Button>
      <Button variant="primary" onClick={() => void acceptClipboard()}>
        {s.clipboardAdd}
      </Button>
    </Banner>
  );
}
