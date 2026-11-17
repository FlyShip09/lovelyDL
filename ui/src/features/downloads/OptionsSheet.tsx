// Options for one download: where it goes, subtitles, a time range, sponsor segments and live
// streams. Starts from the preview's request; defaults live in Settings.

import { useState } from "react";

import { Button } from "../../components/Button";
import { Checkbox, PopUpButton, TextField } from "../../components/controls";
import type { DownloadRequest, MediaInfo, Options, SponsorBlock } from "../../ipc";
import { commands } from "../../ipc";
import { openSheet, updatePreview, useApp } from "../../store/app";
import styles from "../shell/sheets.module.css";

const close = () => openSheet(null);

/** Times as yt-dlp takes them: seconds, or m:ss, or h:mm:ss. */
export function validTime(value: string): boolean {
  return value === "" || /^\d+(:\d{1,2}){0,2}(\.\d+)?$/.test(value.trim());
}

export function OptionsSheet() {
  const preview = useApp((st) => st.preview);
  const defaultDir = useApp((st) => st.settings?.downloadDir ?? "");
  const info: MediaInfo | null = preview?.status.status === "ready" ? preview.status.info : null;
  const [request, setRequest] = useState<DownloadRequest | null>(preview?.request ?? null);
  if (!preview || !request) return null;
  const o = request.options;
  const set = (patch: Partial<Options>) => setRequest({ ...request, options: { ...o, ...patch } });
  const section = o.section ?? { start: "", end: "" };
  const timesOk = validTime(section.start) && validTime(section.end);
  const audio = request.preset.kind === "audio";
  const live = info?.live === "live" || info?.live === "upcoming";

  const apply = () => {
    const empty = section.start.trim() === "" && section.end.trim() === "";
    updatePreview({
      ...request,
      options: { ...o, section: empty ? null : { start: section.start.trim(), end: section.end.trim() } },
    });
    close();
  };

  return (
    <div className={styles.alert}>
      <h2 className={styles.title}>Options for this download</h2>
      <div className={styles.form}>
        <span className={styles.formLabel}>Save to:</span>
        <span className={styles.formValue}>
          <span className={styles.path}>{request.destination ?? defaultDir}</span>
          <Button
            onClick={() =>
              void commands.chooseFolder(request.destination ?? defaultDir).then(async (folder) => {
                if (!folder) return;
                await commands.checkFolder(folder);
                setRequest({ ...request, destination: folder });
              })
            }
          >
            Change…
          </Button>
          {request.destination && (
            <Button variant="plain" onClick={() => setRequest({ ...request, destination: null })}>
              Use default
            </Button>
          )}
        </span>

        {!audio && (
          <>
            <span className={styles.formLabel}>Subtitles:</span>
            <span className={styles.formStack}>
              <Checkbox
                label={
                  info?.subtitles.length
                    ? `Download subtitles (${info.subtitles.slice(0, 6).join(", ")}${info.subtitles.length > 6 ? "…" : ""})`
                    : "Download subtitles"
                }
                checked={o.subtitles}
                onChange={(subtitles) => set({ subtitles })}
              />
              {o.subtitles && (
                <span className={styles.formRow}>
                  <span className={styles.muted}>Languages</span>
                  <TextField
                    aria-label="Subtitle languages"
                    defaultValue={o.subLangs.join(", ")}
                    onBlur={(e) => set({ subLangs: e.target.value.split(/[,\s]+/).filter(Boolean) })}
                  />
                </span>
              )}
              {o.subtitles && info?.autoCaptions && (
                <Checkbox
                  label="Include auto-generated captions"
                  checked={o.autoSubs}
                  onChange={(autoSubs) => set({ autoSubs })}
                />
              )}
            </span>
          </>
        )}

        <span className={styles.formLabel}>Only part of it:</span>
        <span className={styles.formStack}>
          <span className={styles.formRow}>
            <TextField
              aria-label="Start time"
              placeholder="Start, e.g. 1:30"
              value={section.start}
              onChange={(e) => set({ section: { ...section, start: e.target.value } })}
            />
            <span className={styles.muted}>to</span>
            <TextField
              aria-label="End time"
              placeholder="End, e.g. 2:45"
              value={section.end}
              onChange={(e) => set({ section: { ...section, end: e.target.value } })}
            />
          </span>
          {!timesOk && <span className={styles.error}>Use seconds or minutes:seconds, like 90 or 1:30.</span>}
          <Checkbox
            label="Cut exactly at these times (slower, re-encodes the edges)"
            checked={o.preciseCuts}
            onChange={(preciseCuts) => set({ preciseCuts })}
          />
        </span>

        <span className={styles.formLabel}>Sponsor segments:</span>
        <span className={styles.formValue}>
          <PopUpButton<SponsorBlock>
            label="Sponsor segments"
            value={o.sponsorblock}
            onChange={(sponsorblock) => set({ sponsorblock })}
            choices={[
              { value: "off", label: "Keep as they are" },
              { value: "mark", label: "Mark as chapters" },
              { value: "remove", label: "Cut out" },
            ]}
          />
        </span>

        {live && (
          <>
            <span className={styles.formLabel}>Live stream:</span>
            <span className={styles.formStack}>
              <Checkbox
                label="Record from the start of the stream"
                checked={o.liveFromStart}
                onChange={(liveFromStart) => set({ liveFromStart })}
              />
              <Checkbox
                label="If it has not started, wait for it"
                checked={o.waitForLive}
                onChange={(waitForLive) => set({ waitForLive })}
              />
            </span>
          </>
        )}
      </div>
      <div className={styles.buttons}>
        <Button onClick={close}>Cancel</Button>
        <Button variant="primary" disabled={!timesOk} onClick={apply}>
          Use these options
        </Button>
      </div>
    </div>
  );
}
