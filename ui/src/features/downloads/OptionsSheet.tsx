// Options for one download: where it goes, subtitles, a time range, sponsor segments and live
// streams. Starts from the preview's request; defaults live in Settings.

import { useState } from "react";

import { Button } from "../../components/Button";
import { Checkbox, PopUpButton, TextField } from "../../components/controls";
import type { DownloadRequest, MediaInfo, Options, SponsorBlock } from "../../ipc";
import { commands } from "../../ipc";
import { openSheet, updatePreview, useApp } from "../../store/app";
import { s } from "../../strings";
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
      <h2 className={styles.title}>{s.optionsTitle}</h2>
      <div className={styles.form}>
        <span className={styles.formLabel}>{s.saveToRow}</span>
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
            {s.changeDestination}
          </Button>
          {request.destination && (
            <Button variant="plain" onClick={() => setRequest({ ...request, destination: null })}>
              {s.useDefault}
            </Button>
          )}
        </span>

        {!audio && (
          <>
            <span className={styles.formLabel}>{s.subtitlesRow}</span>
            <span className={styles.formStack}>
              <Checkbox
                label={
                  info?.subtitles.length
                    ? s.downloadSubtitlesWith(
                        `${info.subtitles.slice(0, 6).join(", ")}${info.subtitles.length > 6 ? "…" : ""}`,
                      )
                    : s.downloadSubtitles
                }
                checked={o.subtitles}
                onChange={(subtitles) => set({ subtitles })}
              />
              {o.subtitles && (
                <span className={styles.formRow}>
                  <span className={styles.muted}>{s.languages}</span>
                  <TextField
                    aria-label={s.subtitleLanguages}
                    defaultValue={o.subLangs.join(", ")}
                    onBlur={(e) => set({ subLangs: e.target.value.split(/[,\s]+/).filter(Boolean) })}
                  />
                </span>
              )}
              {o.subtitles && info?.autoCaptions && (
                <Checkbox
                  label={s.autoCaptions}
                  checked={o.autoSubs}
                  onChange={(autoSubs) => set({ autoSubs })}
                />
              )}
            </span>
          </>
        )}

        <span className={styles.formLabel}>{s.partRow}</span>
        <span className={styles.formStack}>
          <span className={styles.formRow}>
            <TextField
              aria-label={s.startTime}
              placeholder={s.startPlaceholder}
              value={section.start}
              onChange={(e) => set({ section: { ...section, start: e.target.value } })}
            />
            <span className={styles.muted}>{s.timeTo}</span>
            <TextField
              aria-label={s.endTime}
              placeholder={s.endPlaceholder}
              value={section.end}
              onChange={(e) => set({ section: { ...section, end: e.target.value } })}
            />
          </span>
          {!timesOk && <span className={styles.error}>{s.timeFormatError}</span>}
          <Checkbox
            label={s.preciseCuts}
            checked={o.preciseCuts}
            onChange={(preciseCuts) => set({ preciseCuts })}
          />
        </span>

        <span className={styles.formLabel}>{s.sponsorSegmentsRow}</span>
        <span className={styles.formValue}>
          <PopUpButton<SponsorBlock>
            label={s.sponsorSegmentsLabel}
            value={o.sponsorblock}
            onChange={(sponsorblock) => set({ sponsorblock })}
            choices={[
              { value: "off", label: s.sponsorKeep },
              { value: "mark", label: s.sponsorMark },
              { value: "remove", label: s.sponsorRemove },
            ]}
          />
        </span>

        {live && (
          <>
            <span className={styles.formLabel}>{s.liveRow}</span>
            <span className={styles.formStack}>
              <Checkbox
                label={s.liveFromStart}
                checked={o.liveFromStart}
                onChange={(liveFromStart) => set({ liveFromStart })}
              />
              <Checkbox
                label={s.waitForLive}
                checked={o.waitForLive}
                onChange={(waitForLive) => set({ waitForLive })}
              />
            </span>
          </>
        )}
      </div>
      <div className={styles.buttons}>
        <Button onClick={close}>{s.cancelButton}</Button>
        <Button variant="primary" disabled={!timesOk} onClick={apply}>
          {s.useOptions}
        </Button>
      </div>
    </div>
  );
}
