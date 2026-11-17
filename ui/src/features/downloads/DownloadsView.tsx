import { X } from "lucide-react";
import { AnimatePresence, motion, Reorder } from "motion/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { IconButton } from "../../components/Button";
import type { Job } from "../../ipc";
import { commands } from "../../ipc";
import { fade, smooth } from "../../motion/tokens";
import { clearLinkError, focusHandled, select, startPreview, useApp } from "../../store/app";
import { groupJobs, isActive, type SectionId, sectionOf, sectionTitle } from "../../store/derive";
import { s } from "../../strings";
import { primaryAction } from "./actions";
import styles from "./downloads.module.css";
import { JobRow } from "./JobRow";
import { PreviewCard } from "./PreviewCard";

const ranks: SectionId[] = ["active", "queued", "paused", "failed", "canceled", "completed"];

/**
 * Sections as shown. A row that just finished stays with the active ones for a beat, and while
 * the pointer is over the list nothing changes section, so rows never move under the hand.
 */
function arrange(order: string[], jobs: Record<string, Job>, place: (job: Job) => SectionId) {
  const sections = new Map<SectionId, Job[]>();
  for (const job of groupJobs(order, jobs).flatMap((g) => g.jobs)) {
    const id = place(job);
    sections.set(id, [...(sections.get(id) ?? []), job]);
  }
  return ranks.flatMap((id) => {
    const list = sections.get(id);
    return list ? [{ id, jobs: list }] : [];
  });
}

function Empty({ failedOnly }: { failedOnly: boolean }) {
  return (
    <div className={styles.empty}>
      <span className={styles.emptyTitle}>{failedOnly ? s.emptyFailedTitle : s.emptyTitle}</span>
      <span className={styles.emptyBody}>{failedOnly ? s.emptyFailedBody : s.emptyBody}</span>
    </div>
  );
}

export function DownloadsView({ failedOnly }: { failedOnly: boolean }) {
  const jobs = useApp((st) => st.jobs);
  const order = useApp((st) => st.order);
  const preview = useApp((st) => st.preview);
  const selection = useApp((st) => st.selection);
  const linkError = useApp((st) => st.linkError);
  const focusJob = useApp((st) => st.focusJob);
  const justCompleted = useApp((st) => st.justCompleted);
  const [frozen, setFrozen] = useState<Map<string, SectionId> | null>(null);
  // While a queued row is dragged, its section shows this order; the engine gets it on drop.
  const [dragOrder, setDragOrder] = useState<string[] | null>(null);
  const list = useRef<HTMLDivElement>(null);

  const place = (job: Job): SectionId =>
    frozen?.get(job.id) ?? (justCompleted.includes(job.id) ? "active" : sectionOf(job.state));
  const all = arrange(order, jobs, place);
  // Rows keep their sections while the pointer moves over the list, so nothing jumps under a
  // click. Two seconds of stillness, or leaving the list, lets them settle.
  const idle = useRef(0);
  const hold = () => {
    setFrozen((f) => f ?? new Map(all.flatMap((x) => x.jobs.map((j): [string, SectionId] => [j.id, x.id]))));
    window.clearTimeout(idle.current);
    idle.current = window.setTimeout(() => setFrozen(null), 2000);
  };
  const release = () => {
    window.clearTimeout(idle.current);
    setFrozen(null);
  };
  const sections = failedOnly ? all.filter((x) => x.id === "failed") : all;
  const flat = useMemo(() => sections.flatMap((x) => x.jobs.map((j) => j.id)), [sections]);

  /** Puts the dragged order of the queued section back into the whole queue and saves it. */
  const commitOrder = (queued: string[]) => {
    const next = dragOrder;
    setDragOrder(null);
    if (!next || next.join() === queued.join()) return;
    let i = 0;
    const whole = order.map((id) => (queued.includes(id) ? (next[i++] ?? id) : id));
    useApp.setState({ order: whole });
    void commands.jobsReorder(whole);
  };

  const onSelect = useCallback(
    (id: string, how: "only" | "toggle" | "range") => {
      const current = useApp.getState().selection;
      if (how === "toggle") select(current.includes(id) ? current.filter((x) => x !== id) : [...current, id]);
      else if (how === "range" && current.length > 0) {
        const anchor = flat.indexOf(current[0] ?? id);
        const end = flat.indexOf(id);
        const [a, b] = anchor < end ? [anchor, end] : [end, anchor];
        select([current[0] ?? id, ...flat.slice(a, b + 1).filter((x) => x !== current[0])]);
      } else select([id]);
    },
    [flat],
  );

  // A notification was clicked: bring that row into view.
  useEffect(() => {
    if (!focusJob) return;
    list.current
      ?.querySelector<HTMLElement>(`[data-job="${focusJob}"]`)
      ?.scrollIntoView({ block: "nearest" });
    list.current?.querySelector<HTMLElement>(`[data-job="${focusJob}"]`)?.focus();
    focusHandled();
  }, [focusJob]);

  // Return starts the download shown in the preview, unless focus is somewhere that uses Return.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const state = useApp.getState();
      if (
        e.key !== "Enter" ||
        state.sheet ||
        state.preview?.status.status !== "ready" ||
        state.preview.status.info.drm
      )
        return;
      const target = e.target instanceof Element ? e.target : null;
      if (target?.closest("button, input, textarea, [role=row], [role=menu]")) return;
      e.preventDefault();
      void startPreview();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const onKeyDown = (e: React.KeyboardEvent) => {
    const current = selection[0];
    const index = current ? flat.indexOf(current) : -1;
    const move = (to: number) => {
      const id = flat[Math.max(0, Math.min(flat.length - 1, to))];
      if (!id) return;
      select([id]);
      list.current?.querySelector<HTMLElement>(`[data-job="${id}"]`)?.focus();
    };
    if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      // The keyboard's way to reorder: Alt+arrows move a queued download one place.
      const queued = all.find((x) => x.id === "queued")?.jobs.map((j) => j.id) ?? [];
      const at = current ? queued.indexOf(current) : -1;
      const to = at + (e.key === "ArrowUp" ? -1 : 1);
      const other = queued[to];
      if (current && at >= 0 && other) {
        const next = [...queued];
        next[at] = other;
        next[to] = current;
        let i = 0;
        const whole = order.map((id) => (queued.includes(id) ? (next[i++] ?? id) : id));
        useApp.setState({ order: whole });
        void commands.jobsReorder(whole);
      }
    } else if (e.key === "ArrowDown") move(index + 1);
    else if (e.key === "ArrowUp") move(index - 1);
    else if (e.key === "Home") move(0);
    else if (e.key === "End") move(flat.length - 1);
    else if (e.key === "Delete" || e.key === "Backspace") {
      const doomed = selection.filter((id) => jobs[id] && !isActive(jobs[id].state));
      selection
        .filter((id) => jobs[id] && isActive(jobs[id].state))
        .forEach((id) => void commands.jobCancel(id));
      if (doomed.length) void commands.jobRemove(doomed);
    } else if (e.key === " " || e.key === "Enter") {
      selection.forEach((id) => {
        const job = jobs[id];
        if (job) primaryAction(job)?.();
      });
    } else if (e.key === "a" && e.ctrlKey) select(flat);
    else return;
    e.preventDefault();
  };

  const empty = flat.length === 0 && (failedOnly || !preview);

  return (
    <div className={styles.scroll} onMouseDown={(e) => e.target === e.currentTarget && select([])}>
      <AnimatePresence>
        {linkError && (
          <motion.div
            className={styles.linkError}
            role="alert"
            initial={{ opacity: 0, y: -6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            transition={fade}
          >
            <span className={styles.linkErrorText}>{linkError}</span>
            <IconButton small icon={X} label={s.close} onClick={clearLinkError} />
          </motion.div>
        )}
      </AnimatePresence>
      {!failedOnly && <PreviewCard />}
      {empty ? (
        <Empty failedOnly={failedOnly} />
      ) : (
        <motion.div
          ref={list}
          layout="position"
          transition={smooth}
          role="grid"
          aria-multiselectable
          aria-label={failedOnly ? s.failed : s.downloads}
          className={styles.list}
          onPointerEnter={hold}
          onPointerMove={hold}
          onPointerLeave={release}
          onKeyDown={onKeyDown}
          onFocus={(e) => {
            if (e.target === e.currentTarget && flat[0] && selection.length === 0) select([flat[0]]);
          }}
          tabIndex={selection.length === 0 ? 0 : -1}
        >
          <AnimatePresence initial={false}>
            {sections.map((section) => (
              <motion.div
                key={section.id}
                layout="position"
                transition={smooth}
                className={styles.section}
                role="rowgroup"
                aria-label={sectionTitle[section.id]}
              >
                {!failedOnly && (
                  <div className={styles.sectionTitle} aria-hidden>
                    {sectionTitle[section.id]}
                  </div>
                )}
                {section.id === "queued" && section.jobs.length > 1 && !failedOnly ? (
                  <Reorder.Group
                    as="div"
                    axis="y"
                    role="presentation"
                    values={dragOrder ?? section.jobs.map((j) => j.id)}
                    onReorder={setDragOrder}
                  >
                    {(dragOrder ?? section.jobs.map((j) => j.id)).map((id) => {
                      const job = jobs[id];
                      if (!job) return null;
                      return (
                        <JobRow
                          key={id}
                          job={job}
                          selected={selection.includes(job.id)}
                          selection={selection}
                          onSelect={onSelect}
                          onReorderEnd={() => commitOrder(section.jobs.map((j) => j.id))}
                        />
                      );
                    })}
                  </Reorder.Group>
                ) : (
                  <AnimatePresence initial={false}>
                    {section.jobs.map((job, i) => (
                      <motion.div
                        key={job.id}
                        role="presentation"
                        layout="position"
                        initial={{ opacity: 0, height: 0 }}
                        animate={{ opacity: 1, height: "auto" }}
                        exit={{
                          opacity: 0,
                          height: 0,
                          transition: { ...smooth, delay: Math.min(i, 6) * 0.02 },
                        }}
                        transition={smooth}
                      >
                        <JobRow
                          job={job}
                          selected={selection.includes(job.id)}
                          selection={selection}
                          onSelect={onSelect}
                        />
                      </motion.div>
                    ))}
                  </AnimatePresence>
                )}
              </motion.div>
            ))}
          </AnimatePresence>
        </motion.div>
      )}
    </div>
  );
}
