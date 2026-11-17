import { FolderOpen, Pause, Play, RotateCw, X } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { memo, useEffect, useRef, useState } from "react";

import { Button, IconButton } from "../../components/Button";
import { ProgressBar } from "../../components/controls";
import { openMenu } from "../../components/Menu";
import type { Job } from "../../ipc";
import { commands } from "../../ipc";
import { fade, smooth } from "../../motion/tokens";
import { useApp } from "../../store/app";
import { formatClock, formatSpeed, isActive, rowLabel, statusLine } from "../../store/derive";
import { errorCopy, fixLabel, s } from "../../strings";
import { applyFix, copy, jobMenu } from "./actions";
import styles from "./downloads.module.css";
import { Thumb } from "./Thumb";

/** Speed and time left change at most twice a second, so they stay readable. */
function useSteady<T>(value: T, ms = 500): T {
  const [shown, setShown] = useState(value);
  const last = useRef(0);
  useEffect(() => {
    if (Object.is(value, shown)) return;
    const timer = window.setTimeout(
      () => {
        last.current = Date.now();
        setShown(value);
      },
      Math.max(0, last.current + ms - Date.now()),
    );
    return () => window.clearTimeout(timer);
  }, [value, shown, ms]);
  return shown;
}

function JustDone() {
  return (
    <svg className={styles.check} viewBox="0 0 22 22" aria-hidden>
      <motion.path
        d="M6 11.5l3.2 3.2L16 8"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        initial={{ pathLength: 0 }}
        animate={{ pathLength: 1 }}
        transition={{ duration: 0.35, ease: "easeOut" }}
      />
    </svg>
  );
}

interface Props {
  job: Job;
  selected: boolean;
  selection: string[];
  onSelect: (id: string, how: "only" | "toggle" | "range") => void;
}

export const JobRow = memo(function JobRow({ job, selected, selection, onSelect }: Props) {
  const p = job.progress;
  const active = isActive(job.state);
  const speed = useSteady(job.state === "downloading" ? formatSpeed(p.speed) : "");
  const eta = useSteady(job.state === "downloading" ? formatClock(p.eta) : "");
  const percent =
    active && p.fraction !== null && job.state === "downloading" ? `${Math.round(p.fraction * 100)}%` : "";
  const showCheck = useApp((st) => job.state === "completed" && st.justCompleted.includes(job.id));

  const error = job.state === "failed" ? job.error : null;
  const copyText = error ? errorCopy[error.kind] : null;

  return (
    <div
      role="row"
      aria-selected={selected}
      aria-label={rowLabel(job)}
      tabIndex={selected ? 0 : -1}
      data-job={job.id}
      className={styles.row}
      onMouseDown={(e) => {
        if (e.button !== 0) return;
        onSelect(job.id, e.shiftKey ? "range" : e.ctrlKey ? "toggle" : "only");
      }}
      onDoubleClick={() => job.state === "completed" && void commands.openFile(job.id)}
      onContextMenu={(e) => {
        e.preventDefault();
        if (!selected) onSelect(job.id, "only");
        openMenu(jobMenu(job, selected ? selection : [job.id]), { x: e.clientX, y: e.clientY });
      }}
    >
      <Thumb src={job.thumbnail} kind={job.kind} state={job.state} fraction={p.fraction} />
      <div className={styles.text} role="gridcell">
        <span className={`${styles.title} selectable`}>{job.title || job.url}</span>
        {job.state === "downloading" && (
          <ProgressBar value={p.fraction} stage={`${p.stage}${p.stageIndex}`} label={rowLabel(job)} />
        )}
        <span className={styles.sub}>
          <AnimatePresence mode="popLayout" initial={false}>
            <motion.span
              key={job.state === "failed" ? "failed" : `${job.state}:${p.stage}`}
              className={`${styles.status} ${error ? styles.failedStatus : ""}`}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={fade}
            >
              {error && copyText ? copyText.title : job.fileMissing ? s.fileMissing : statusLine(job)}
            </motion.span>
          </AnimatePresence>
          {(percent || speed || eta) && (
            <span className={styles.numbers}>
              <span>{percent}</span>
              {speed && <span>{speed}</span>}
              {eta && <span>{eta}</span>}
            </span>
          )}
        </span>
      </div>
      <div className={styles.actions} role="gridcell">
        {(active || job.state === "queued") && (
          <IconButton small icon={Pause} label={s.pause} onClick={() => void commands.jobPause(job.id)} />
        )}
        {job.state === "paused" && (
          <IconButton small icon={Play} label={s.resume} onClick={() => void commands.jobResume(job.id)} />
        )}
        {(active || job.state === "queued" || job.state === "paused") && (
          <IconButton small icon={X} label={s.cancel} onClick={() => void commands.jobCancel(job.id)} />
        )}
        {(job.state === "canceled" || (job.state === "failed" && job.error?.retryable)) && (
          <IconButton small icon={RotateCw} label={s.retry} onClick={() => void commands.jobRetry(job.id)} />
        )}
        {(job.state === "canceled" || job.state === "failed") && (
          <IconButton small icon={X} label={s.remove} onClick={() => void commands.jobRemove([job.id])} />
        )}
        {job.state === "completed" &&
          (showCheck ? (
            <JustDone />
          ) : (
            <IconButton
              small
              icon={FolderOpen}
              label={s.showInExplorer}
              disabled={job.fileMissing}
              onClick={() => void commands.reveal(job.id)}
            />
          ))}
      </div>
      <AnimatePresence initial={false}>
        {error && copyText && (
          <motion.div
            key="error"
            role="gridcell"
            className={styles.errorPanel}
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={smooth}
          >
            <div className={styles.errorInner}>
              <span className={styles.errorBody}>{copyText.body}</span>
              <div className={styles.errorButtons} onMouseDown={(e) => e.stopPropagation()}>
                {error.fix && (
                  <Button variant="primary" onClick={() => void applyFix(job)}>
                    {fixLabel[error.fix]}
                  </Button>
                )}
                <Button onClick={() => void copy(commands.jobDetails(job.id))}>{s.copyDetails}</Button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
});
