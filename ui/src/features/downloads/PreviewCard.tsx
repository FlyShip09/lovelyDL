import { X } from "lucide-react";
import { AnimatePresence, motion, useIsPresent } from "motion/react";
import { type Ref, useEffect, useState } from "react";

import { Button, IconButton } from "../../components/Button";
import { type Choice, PopUpButton, Spinner } from "../../components/controls";
import type { DownloadRequest, MediaInfo, Preset, Preview, Resolved } from "../../ipc";
import { commands } from "../../ipc";
import { fade, gentle } from "../../motion/tokens";
import {
  choosePreview,
  dismissPreview,
  openSheet,
  startPreview,
  updatePreview,
  useApp,
} from "../../store/app";
import { formatClock, heightName, resolvedSize, resolvedSummary, topsOutNote } from "../../store/derive";
import { errorBody, errorCopy, fixLabel, s } from "../../strings";
import { copy } from "./actions";
import styles from "./downloads.module.css";
import { Thumb } from "./Thumb";

type FormatKey = "best" | "mp4" | "audio:m4a" | "audio:mp3" | "audio:opus" | "audio:flac";

const formatChoices: (Choice<FormatKey> | "separator")[] = [
  { value: "mp4", label: s.compatibleMp4 },
  { value: "best", label: s.bestQuality },
  "separator",
  { value: "audio:m4a", label: `${s.audioOnly}: M4A` },
  { value: "audio:mp3", label: `${s.audioOnly}: MP3` },
  { value: "audio:opus", label: `${s.audioOnly}: Opus` },
  { value: "audio:flac", label: `${s.audioOnly}: FLAC` },
];

const heights = [null, 2160, 1440, 1080, 720, 480] as const;

function formatKey(p: Preset): FormatKey {
  return p.kind === "audio" ? `audio:${p.format}` : p.kind;
}

function presetFor(key: FormatKey, maxHeight: number | null): Preset {
  if (key === "best" || key === "mp4") return { kind: key, maxHeight };
  const format = key.slice(6) as "m4a" | "mp3" | "opus" | "flac";
  return { kind: "audio", format };
}

/** The format and quality pop-ups, shared by the preview card and the playlist sheet. */
export function PresetControls({
  request,
  onChange,
}: {
  request: DownloadRequest;
  onChange: (r: DownloadRequest) => void;
}) {
  const preset = request.preset;
  const maxHeight = preset.kind === "audio" ? null : preset.maxHeight;
  return (
    <>
      <PopUpButton
        label={s.formatLabel}
        value={formatKey(preset)}
        choices={formatChoices}
        onChange={(key) => onChange({ ...request, formats: null, preset: presetFor(key, maxHeight) })}
      />
      {preset.kind !== "audio" && (
        <PopUpButton
          label={s.qualityLabel}
          value={maxHeight}
          choices={heights.map((h) => ({ value: h, label: heightName(h) }))}
          onChange={(h) => onChange({ ...request, formats: null, preset: presetFor(formatKey(preset), h) })}
        />
      )}
    </>
  );
}

function Flags({ info }: { info: MediaInfo }) {
  const flags: [string, boolean][] = [];
  if (info.live === "live") flags.push([s.live, true]);
  if (info.live === "upcoming") flags.push([s.upcoming, false]);
  if (info.ageRestricted) flags.push([s.ageRestricted, false]);
  if (info.membersOnly) flags.push([s.membersOnly, false]);
  if (info.drm) flags.push([s.drm, true]);
  if (flags.length === 0) return null;
  return (
    <span className={styles.flags}>
      {flags.map(([label, red]) => (
        <span key={label} className={`${styles.flag} ${red ? styles.flagRed : ""}`}>
          {label}
        </span>
      ))}
    </span>
  );
}

function Ready({ preview, info, resolved }: { preview: Preview; info: MediaInfo; resolved: Resolved }) {
  const note = topsOutNote(
    preview.request.preset,
    resolved,
    info.formats.map((f) => f.height),
  );
  const kind = preview.request.preset.kind === "audio" ? "audio" : "video";
  return (
    <>
      <div className={styles.cardTop}>
        <Thumb large src={info.thumbnail} kind={kind} state="completed" fraction={1} />
        <div className={styles.cardText}>
          <span className={`${styles.cardTitle} selectable`}>{info.title}</span>
          <span className={styles.meta}>
            {info.uploader && <span>{info.uploader}</span>}
            {info.duration !== null && <span className="tabular">{formatClock(info.duration)}</span>}
            {info.site && <span>{info.site}</span>}
          </span>
          <Flags info={info} />
          <AnimatePresence mode="wait" initial={false}>
            <motion.span
              key={resolvedSummary(resolved) + (note ?? "")}
              className={styles.resolved}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={fade}
            >
              {info.drm ? s.drmBody : [resolvedSummary(resolved), note].filter(Boolean).join(" — ")}
            </motion.span>
          </AnimatePresence>
        </div>
        <IconButton
          small
          icon={X}
          label={s.dismissPreview}
          className={styles.cardClose}
          onClick={dismissPreview}
        />
      </div>
      {!info.drm && (
        <div className={styles.controls}>
          <PresetControls request={preview.request} onChange={updatePreview} />
          <Button onClick={() => openSheet({ kind: "options" })}>{s.options}…</Button>
          <Button onClick={() => openSheet({ kind: "formats", jobId: null })}>{s.advanced}</Button>
          <span className={styles.spacer} />
          <AnimatePresence mode="wait" initial={false}>
            <motion.span
              key={resolvedSize(resolved, preview.request.options.section, info.duration)}
              className={styles.size}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={fade}
            >
              {resolvedSize(resolved, preview.request.options.section, info.duration)}
            </motion.span>
          </AnimatePresence>
          <Button variant="primary" data-autofocus onClick={() => void startPreview()}>
            {s.download}
          </Button>
        </div>
      )}
    </>
  );
}

function Probing() {
  // Under 300 ms the activity line in the link field is enough; a card would only flicker.
  const [show, setShow] = useState(false);
  useEffect(() => {
    const timer = window.setTimeout(() => setShow(true), 300);
    return () => window.clearTimeout(timer);
  }, []);
  // The card already has the ready card's shape, so nothing jumps when the details arrive.
  return (
    <div aria-busy="true">
      <div className={styles.cardTop}>
        <span className={`${styles.thumb} ${styles.thumbLarge}`} aria-hidden />
        <div className={styles.cardText}>
          {show && (
            <span className={styles.waiting}>
              <Spinner />
              <span>{s.gettingDetails}</span>
            </span>
          )}
        </div>
        <IconButton
          small
          icon={X}
          label={s.dismissPreview}
          className={styles.cardClose}
          onClick={dismissPreview}
        />
      </div>
      <div className={styles.controls} aria-hidden>
        <span className={styles.controlSpace} />
      </div>
    </div>
  );
}

function Body({ preview }: { preview: Preview }) {
  const status = preview.status;
  switch (status.status) {
    case "probing":
      return <Probing />;
    case "ambiguous":
      return (
        <div className={styles.question}>
          <span className={styles.questionText}>
            <span className={styles.errorTitle}>{s.ambiguousTitle}</span>
            <span className={styles.errorBody}>{s.ambiguousBody}</span>
          </span>
          <Button onClick={() => choosePreview("playlist")}>{s.wholePlaylist}</Button>
          <Button variant="primary" data-autofocus onClick={() => choosePreview("video")}>
            {s.justThisVideo}
          </Button>
        </div>
      );
    case "failed": {
      const copyText = errorCopy[status.error.kind];
      const fix = status.error.fix;
      return (
        <div className={styles.question}>
          <span className={styles.questionText}>
            <span className={styles.errorTitle}>{copyText.title}</span>
            <span className={`${styles.errorBody} selectable`}>
              {errorBody(status.error.kind, status.error.detail)}
            </span>
          </span>
          <Button onClick={() => void copy(commands.jobDetails(preview.id))}>{s.copyDetails}</Button>
          {fix === "useCookies" || fix === "installComponents" || fix === "updateYtdlp" ? (
            <Button variant="primary" onClick={() => void commands.openSettings()}>
              {fixLabel[fix]}
            </Button>
          ) : null}
          <IconButton small icon={X} label={s.dismissPreview} onClick={dismissPreview} />
        </div>
      );
    }
    case "ready":
      return <Ready preview={preview} info={status.info} resolved={status.resolved} />;
    case "playlist":
      return null;
  }
}

export function PreviewCard() {
  const preview = useApp((st) => st.preview);
  const visible = preview !== null && preview.status.status !== "playlist";
  return (
    // popLayout: a card on its way out stops taking space at once, so the list below glides up.
    <AnimatePresence mode="popLayout" initial={false}>
      {visible && <Card key={preview.id} preview={preview} />}
    </AnimatePresence>
  );
}

/** `ref` comes from AnimatePresence's popLayout, which measures the card as it leaves. */
function Card({ preview, ref }: { preview: Preview; ref?: Ref<HTMLElement> }) {
  // On its way out the card stays visible but takes no more clicks, so Download cannot fire twice.
  const present = useIsPresent();
  return (
    <motion.section
      ref={ref}
      inert={!present}
      layout="position"
      aria-label={preview.status.status === "ready" ? preview.status.info.title : s.gettingDetails}
      className={styles.card}
      initial={{ opacity: 0, y: -14, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: 24, scale: 0.97, transition: { duration: 0.22 } }}
      // A link still being read waits 300 ms before its card shows, so fast answers never flicker.
      transition={{ ...gentle, delay: preview.status.status === "probing" ? 0.3 : 0 }}
    >
      <Body preview={preview} />
    </motion.section>
  );
}
