import { useVirtualizer } from "@tanstack/react-virtual";
import { FolderOpen } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import { IconButton } from "../../components/Button";
import { SearchField, SegmentedControl } from "../../components/controls";
import { type MenuItem, openMenu } from "../../components/Menu";
import type { HistoryFilter, Job } from "../../ipc";
import { commands } from "../../ipc";
import { useApp } from "../../store/app";
import { formatBytes } from "../../store/derive";
import { errorCopy, s } from "../../strings";
import { copy } from "../downloads/actions";
import rowStyles from "../downloads/downloads.module.css";
import { startDragOut } from "../downloads/JobRow";
import { Thumb } from "../downloads/Thumb";
import styles from "./history.module.css";

const PAGE = 200;
const ROW = 56;

const day = new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short" });
const year = new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short", year: "numeric" });
const time = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });

export function when(ms: number | null): string {
  if (ms === null) return "";
  const date = new Date(ms);
  const today = new Date();
  const startOf = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((startOf(today) - startOf(date)) / 86_400_000);
  if (days === 0) return `Today ${time.format(date)}`;
  if (days === 1) return `Yesterday ${time.format(date)}`;
  return date.getFullYear() === today.getFullYear() ? day.format(date) : year.format(date);
}

function menu(job: Job, reload: () => void): MenuItem[] {
  const done = job.state === "completed" && !job.fileMissing;
  return [
    { label: s.open, disabled: !done, onSelect: () => void commands.openFile(job.id) },
    { label: s.showInExplorer, disabled: !done, onSelect: () => void commands.reveal(job.id) },
    { separator: true },
    { label: s.copyLink, onSelect: () => void commands.copyLink(job.id) },
    { label: s.downloadAgain, onSelect: () => void commands.downloadAgain(job.id) },
    ...(job.state === "failed"
      ? [{ label: s.copyDetails, onSelect: () => void copy(commands.jobDetails(job.id)) }]
      : []),
    { separator: true },
    {
      label: s.removeFromHistory,
      onSelect: () => void Promise.resolve(commands.historyRemove([job.id])).then(reload),
    },
    {
      label: s.moveToRecycleBin,
      destructive: true,
      disabled: !done,
      onSelect: () => void commands.historyTrash([job.id]).then(reload),
    },
  ];
}

export function HistoryView() {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<HistoryFilter>("all");
  const [items, setItems] = useState<Job[]>([]);
  const [total, setTotal] = useState(0);
  const [loaded, setLoaded] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const version = useApp((st) => st.historyVersion);
  const scroll = useRef<HTMLDivElement>(null);
  const loading = useRef(false);

  const load = useCallback(
    async (offset: number) => {
      loading.current = true;
      const page = await commands.historyQuery(query, filter, offset, PAGE);
      setItems((prev) => (offset === 0 ? page.items : [...prev, ...page.items]));
      setTotal(page.total);
      setLoaded(true);
      loading.current = false;
    },
    [query, filter],
  );

  // Typing settles for a moment before the database is asked.
  useEffect(() => {
    const timer = window.setTimeout(() => void load(0), query ? 120 : 0);
    return () => window.clearTimeout(timer);
  }, [load, query, version]);

  const virtual = useVirtualizer({
    count: items.length,
    getScrollElement: () => scroll.current,
    estimateSize: () => ROW,
    overscan: 8,
  });
  const range = virtual.getVirtualItems();
  const last = range[range.length - 1];
  useEffect(() => {
    if (last && last.index > items.length - 20 && items.length < total && !loading.current)
      void load(items.length);
  }, [last, items.length, total, load]);

  const reload = () => void load(0);
  const empty = loaded && items.length === 0;

  return (
    <div className={styles.view}>
      <div className={styles.bar}>
        <SearchField
          data-history-search
          label={s.searchHistory}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Escape" && setQuery("")}
          className={styles.search}
        />
        <SegmentedControl
          label="Show"
          value={filter}
          onChange={setFilter}
          choices={[
            { value: "all", label: s.filterAll },
            { value: "video", label: s.filterVideo },
            { value: "audio", label: s.filterAudio },
            { value: "failed", label: s.filterFailed },
          ]}
        />
      </div>
      {empty ? (
        <div className={rowStyles.empty}>
          <span className={rowStyles.emptyTitle}>
            {query || filter !== "all" ? s.noMatches : s.emptyHistoryTitle}
          </span>
          {!query && filter === "all" && <span className={rowStyles.emptyBody}>{s.emptyHistoryBody}</span>}
        </div>
      ) : (
        <div ref={scroll} className={styles.scroll}>
          <div
            role="grid"
            aria-label={s.history}
            className={`${rowStyles.list} ${styles.list}`}
            style={{ height: virtual.getTotalSize() }}
            onKeyDown={(e) => {
              const index = items.findIndex((j) => j.id === selected);
              const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
              if (step) {
                e.preventDefault();
                const next = items[Math.max(0, Math.min(items.length - 1, index + step))];
                if (next) {
                  setSelected(next.id);
                  virtual.scrollToIndex(items.indexOf(next));
                  // Focus follows the selection, once the row is rendered, so it never falls out
                  // of the virtual list and keys keep working.
                  const focus = (tries: number) => {
                    const row = scroll.current?.querySelector<HTMLElement>(`[data-history="${next.id}"]`);
                    if (row) row.focus({ preventScroll: true });
                    else if (tries > 0) requestAnimationFrame(() => focus(tries - 1));
                  };
                  requestAnimationFrame(() => focus(3));
                }
              } else if (e.key === "Enter" && selected) void commands.openFile(selected);
              else if (e.key === "Delete" && selected)
                void Promise.resolve(commands.historyRemove([selected])).then(reload);
            }}
          >
            {range.map((row) => {
              const job = items[row.index];
              if (!job) return null;
              const failed = job.state === "failed";
              const sub = failed
                ? job.error
                  ? errorCopy[job.error.kind].title
                  : s.failed
                : job.fileMissing
                  ? s.fileMissing
                  : [when(job.finishedAt), formatBytes(job.bytes), job.site].filter(Boolean).join(" · ");
              return (
                <div
                  key={job.id}
                  role="row"
                  aria-selected={selected === job.id}
                  tabIndex={selected === job.id || (!selected && row.index === 0) ? 0 : -1}
                  data-history={job.id}
                  className={rowStyles.row}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 12,
                    right: 12,
                    transform: `translateY(${row.start}px)`,
                  }}
                  onMouseDown={(e) => {
                    setSelected(job.id);
                    if (!failed && !job.fileMissing && e.button === 0) startDragOut(e, job.id);
                  }}
                  onDoubleClick={() => !failed && !job.fileMissing && void commands.openFile(job.id)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    setSelected(job.id);
                    openMenu(menu(job, reload), { x: e.clientX, y: e.clientY });
                  }}
                >
                  <Thumb
                    src={job.thumbnail}
                    kind={job.kind}
                    state={failed ? "failed" : "completed"}
                    fraction={1}
                  />
                  <div className={rowStyles.text} role="gridcell">
                    <span className={`${rowStyles.title} selectable`}>{job.title || job.url}</span>
                    <span className={rowStyles.sub}>
                      <span className={`${rowStyles.status} ${failed ? rowStyles.failedStatus : ""}`}>
                        {sub}
                      </span>
                      {!failed && <span className={rowStyles.numbers}>{job.uploader}</span>}
                    </span>
                  </div>
                  <div className={rowStyles.actions} role="gridcell">
                    {!failed && (
                      <IconButton
                        small
                        icon={FolderOpen}
                        label={s.showInExplorer}
                        disabled={job.fileMissing}
                        onClick={() => void commands.reveal(job.id)}
                      />
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
