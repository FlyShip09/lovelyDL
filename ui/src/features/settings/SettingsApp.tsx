// The Settings window: toolbar tabs like Safari's, forms with right-aligned labels.

import type { LucideIcon } from "lucide-react";
import { Download, Film, Globe, Package, SlidersHorizontal } from "lucide-react";
import { motion } from "motion/react";
import type { ReactNode } from "react";
import { useEffect, useState } from "react";

import { Button } from "../../components/Button";
import { TrafficLights } from "../../components/chrome";
import { Checkbox, PopUpButton, ProgressBar, TextField, Toggle } from "../../components/controls";
import { Icon } from "../../components/Icon";
import { MenuHost } from "../../components/Menu";
import type { OnAdd, ReleaseChannel, Settings, SponsorBlock, Tool } from "../../ipc";
import { commands } from "../../ipc";
import { snappy } from "../../motion/tokens";
import { saveSettings, useApp, useConnection } from "../../store/app";
import { s, sponsorCategoryLabel } from "../../strings";
import { PresetControls } from "../downloads/PreviewCard";
import styles from "./settings.module.css";

type Tab = "general" | "formats" | "network" | "components" | "advanced";

const tabs: { id: Tab; label: string; icon: LucideIcon }[] = [
  { id: "general", label: s.tabGeneral, icon: SlidersHorizontal },
  { id: "formats", label: s.tabFormats, icon: Film },
  { id: "network", label: s.tabNetwork, icon: Globe },
  { id: "components", label: s.tabComponents, icon: Package },
  { id: "advanced", label: s.tabAdvanced, icon: Download },
];

const change = (patch: Partial<Settings>) => void saveSettings((st) => ({ ...st, ...patch }));

function Row({ label, children, note }: { label: string; children: ReactNode; note?: ReactNode }) {
  return (
    <div className={styles.row}>
      <span className={styles.label}>{label}</span>
      <div className={styles.value}>
        {children}
        {note && <span className={styles.note}>{note}</span>}
      </div>
    </div>
  );
}

function Switch({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <span className={styles.switch}>
      <Toggle checked={checked} onChange={onChange} label={label} />
      <span onClick={() => onChange(!checked)}>{label}</span>
    </span>
  );
}

/** A text field that saves when you press Return or leave it. */
function Field({
  value,
  onSave,
  placeholder,
  wide,
  label,
}: {
  value: string;
  onSave: (v: string) => void;
  placeholder?: string;
  wide?: boolean;
  label: string;
}) {
  // The caller keys this field by its value, so an outside change starts it fresh.
  const [text, setText] = useState(value);
  return (
    <TextField
      aria-label={label}
      value={text}
      placeholder={placeholder}
      className={wide ? styles.wide : styles.medium}
      onChange={(e) => setText(e.target.value)}
      onBlur={() => text !== value && onSave(text)}
      onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
    />
  );
}

const counts = [1, 2, 3, 4, 5, 6, 7, 8].map((n) => ({ value: n, label: String(n) }));

function General({ st }: { st: Settings }) {
  return (
    <>
      <Row label={s.saveDownloadsTo}>
        <span className={styles.path}>{st.downloadDir}</span>
        <Button
          onClick={() =>
            void commands.chooseFolder(st.downloadDir).then(async (folder) => {
              if (!folder) return;
              await commands.checkFolder(folder);
              change({ downloadDir: folder });
            })
          }
        >
          {s.changeDestination}
        </Button>
      </Row>
      <Row label={s.onAddRow}>
        <PopUpButton<OnAdd>
          label={s.onAddLabel}
          value={st.onAdd}
          onChange={(onAdd) => change({ onAdd })}
          choices={[
            { value: "preview", label: s.onAddPreview },
            { value: "download", label: s.onAddDownload },
          ]}
        />
      </Row>
      <Row label={s.concurrentRow} note={s.concurrentNote}>
        <PopUpButton
          label={s.concurrentLabel}
          value={st.maxConcurrent}
          choices={counts}
          onChange={(maxConcurrent) => change({ maxConcurrent })}
        />
        <span className={styles.inline}>{s.perSite}</span>
        <PopUpButton
          label={s.perSiteLabel}
          value={st.maxPerSite}
          choices={counts}
          onChange={(maxPerSite) => change({ maxPerSite })}
        />
      </Row>
      <div className={styles.separator} />
      <Row label={s.notificationsRow}>
        <Switch
          label={s.notificationsSwitch}
          checked={st.notifications}
          onChange={(notifications) => change({ notifications })}
        />
      </Row>
      <Row label={s.clipboardRow} note={s.clipboardNote}>
        <Switch
          label={s.clipboardSwitch}
          checked={st.noticeCopiedLinks}
          onChange={(noticeCopiedLinks) => change({ noticeCopiedLinks })}
        />
      </Row>
      <Row label={s.keepAwakeRow}>
        <Switch
          label={s.keepAwakeSwitch}
          checked={st.keepAwake}
          onChange={(keepAwake) => change({ keepAwake })}
        />
      </Row>
      <Row label={s.trayRow} note={s.trayNote}>
        <Switch label={s.traySwitch} checked={st.trayIcon} onChange={(trayIcon) => change({ trayIcon })} />
      </Row>
    </>
  );
}

const sponsorCategories = Object.entries(sponsorCategoryLabel);

function Formats({ st }: { st: Settings }) {
  const req = st.defaultRequest;
  const o = req.options;
  const setOptions = (patch: Partial<typeof o>) =>
    change({ defaultRequest: { ...req, options: { ...o, ...patch } } });
  const [preview, setPreview] = useState("");
  useEffect(() => {
    commands.templatePreview(st.filenameTemplate).then(setPreview, (e: unknown) => setPreview(String(e)));
  }, [st.filenameTemplate]);
  return (
    <>
      <Row label={s.defaultFormatRow}>
        <PresetControls request={req} onChange={(defaultRequest) => change({ defaultRequest })} />
      </Row>
      <Row label={s.embedRow}>
        <div className={styles.stack}>
          <Checkbox
            label={s.embedThumbnail}
            checked={o.embedThumbnail}
            onChange={(embedThumbnail) => setOptions({ embedThumbnail })}
          />
          <Checkbox
            label={s.embedMetadata}
            checked={o.embedMetadata}
            onChange={(embedMetadata) => setOptions({ embedMetadata })}
          />
          <Checkbox
            label={s.embedChapters}
            checked={o.embedChapters}
            onChange={(embedChapters) => setOptions({ embedChapters })}
          />
        </div>
      </Row>
      <Row label={s.subtitlesRow}>
        <div className={styles.stack}>
          <Checkbox
            label={s.downloadSubtitles}
            checked={o.subtitles}
            onChange={(subtitles) => setOptions({ subtitles })}
          />
          <span className={styles.inlineRow}>
            <span className={styles.inline}>{s.languages}</span>
            <Field
              key={o.subLangs.join(", ")}
              label={s.subtitleLanguages}
              value={o.subLangs.join(", ")}
              placeholder={s.subLangsPlaceholder}
              onSave={(v) => setOptions({ subLangs: v.split(/[,\s]+/).filter(Boolean) })}
            />
          </span>
          <Checkbox
            label={s.autoCaptions}
            checked={o.autoSubs}
            onChange={(autoSubs) => setOptions({ autoSubs })}
          />
          <PopUpButton
            label={s.subtitleFilesLabel}
            value={o.embedSubs}
            onChange={(embedSubs) => setOptions({ embedSubs })}
            choices={[
              { value: true, label: s.embedSubs },
              { value: false, label: s.separateSubs },
            ]}
          />
        </div>
      </Row>
      <Row label={s.sponsorBlockRow} note={s.sponsorBlockNote}>
        <div className={styles.stack}>
          <PopUpButton<SponsorBlock>
            label={s.sponsorBlockLabel}
            value={o.sponsorblock}
            onChange={(sponsorblock) => setOptions({ sponsorblock })}
            choices={[
              { value: "off", label: s.sponsorOff },
              { value: "mark", label: s.sponsorMark },
              { value: "remove", label: s.sponsorRemove },
            ]}
          />
          {o.sponsorblock !== "off" && (
            <div className={styles.grid}>
              {sponsorCategories.map(([id, label]) => (
                <Checkbox
                  key={id}
                  label={label}
                  checked={o.sponsorblockCategories.includes(id)}
                  onChange={(on) =>
                    setOptions({
                      sponsorblockCategories: on
                        ? [...o.sponsorblockCategories, id]
                        : o.sponsorblockCategories.filter((c) => c !== id),
                    })
                  }
                />
              ))}
            </div>
          )}
        </div>
      </Row>
      <div className={styles.separator} />
      <Row
        label={s.fileNamesRow}
        note={
          <>
            {s.fileNameExample} <span className={`${styles.mono} selectable`}>{preview}</span>
          </>
        }
      >
        <Field
          key={st.filenameTemplate}
          label={s.fileNameTemplate}
          wide
          value={st.filenameTemplate}
          onSave={(filenameTemplate) => change({ filenameTemplate })}
        />
      </Row>
      <Row label={s.playlistsRow}>
        <div className={styles.stack}>
          <Checkbox
            label={s.playlistIndex}
            checked={st.playlistIndex}
            onChange={(playlistIndex) => change({ playlistIndex })}
          />
          <Checkbox
            label={s.skipDownloaded}
            checked={st.skipDownloaded}
            onChange={(skipDownloaded) => change({ skipDownloaded })}
          />
        </div>
      </Row>
    </>
  );
}

const browsers = ["", "firefox", "chrome", "edge", "brave", "opera", "vivaldi", "chromium", "whale"];
const browserName = (b: string) => (b ? (b[0]?.toUpperCase() ?? "") + b.slice(1) : s.noCookies);
const bookmarklet = "javascript:location.href='lovelydl://download?url='+encodeURIComponent(location.href)";

function Network({ st }: { st: Settings }) {
  const chromium = ["chrome", "edge", "brave", "opera", "vivaldi", "chromium", "whale"].includes(
    st.cookiesBrowser,
  );
  return (
    <>
      <Row label={s.speedLimitRow} note={s.speedLimitNote}>
        <Field
          key={st.rateLimit}
          label={s.speedLimitLabel}
          value={st.rateLimit}
          placeholder={s.noLimit}
          onSave={(rateLimit) => change({ rateLimit })}
        />
      </Row>
      <Row label={s.proxyRow} note={s.proxyNote}>
        <Field
          key={st.proxy}
          label={s.proxyLabel}
          wide
          value={st.proxy}
          placeholder={s.proxyNone}
          onSave={(proxy) => change({ proxy })}
        />
      </Row>
      <Row label={s.bigPlaylistsRow} note={s.paceNote}>
        <Switch
          label={s.paceSwitch}
          checked={st.paceRequests}
          onChange={(paceRequests) => change({ paceRequests })}
        />
      </Row>
      <div className={styles.separator} />
      <Row
        label={s.cookiesRow}
        note={
          st.cookiesBrowser === "firefox"
            ? s.cookiesNoteFirefox
            : chromium
              ? s.cookiesNoteChromium(browserName(st.cookiesBrowser))
              : s.cookiesNoteNone
        }
      >
        <PopUpButton
          label={s.cookiesLabel}
          value={st.cookiesBrowser}
          onChange={(cookiesBrowser) => change({ cookiesBrowser })}
          choices={browsers.map((b) => ({
            value: b,
            label: b ? s.cookiesFrom(browserName(b)) : browserName(b),
          }))}
        />
      </Row>
      <Row label={s.bookmarkletRow} note={s.bookmarkletNote}>
        <span className={`${styles.mono} ${styles.path} selectable`}>{bookmarklet}</span>
        <Button onClick={() => void commands.copyText(bookmarklet)}>{s.copy}</Button>
      </Row>
    </>
  );
}

const toolInfo: Record<Tool["id"], string> = { ytdlp: "yt-dlp", ffmpeg: s.ffmpegAndFfprobe, deno: "Deno" };

function ToolRow({ tool }: { tool: Tool }) {
  const [error, setError] = useState<string | null>(null);
  const status = tool.status;
  const run = (p: Promise<unknown>) =>
    p.then(
      () => setError(null),
      (e: unknown) => setError(String(e)),
    );
  return (
    <div className={styles.tool}>
      <div className={styles.toolText}>
        <span className={styles.toolName}>{toolInfo[tool.id]}</span>
        <span className={styles.note}>
          {status.status === "ready" &&
            `${tool.version} · ${tool.origin === "managed" ? s.originManaged : tool.origin === "system" ? s.originSystem : s.originCustom}`}
          {status.status === "missing" && s.notInstalled}
          {status.status === "checking" && s.checking}
          {status.status === "broken" && s.toolBrokenDetail(status.message)}
          {tool.latest && status.status === "ready" && ` · ${s.versionAvailable(tool.latest)}`}
        </span>
        {error && <span className={styles.error}>{error}</span>}
        {status.status === "installing" && (
          <ProgressBar value={status.fraction} label={s.installingTool(toolInfo[tool.id])} />
        )}
      </div>
      {status.status !== "installing" && (tool.origin === "managed" || status.status !== "ready") && (
        <>
          {status.status === "ready" && (
            <Button onClick={() => void run(commands.componentRollback(tool.id))}>{s.previousVersion}</Button>
          )}
          <Button onClick={() => void run(commands.componentInstall(tool.id))}>
            {status.status === "ready" ? s.update : s.install}
          </Button>
        </>
      )}
    </div>
  );
}

function Components({ st }: { st: Settings }) {
  const components = useApp((state) => state.components);
  return (
    <>
      <div className={styles.tools}>
        {components &&
          [components.ytdlp, components.ffmpeg, components.deno].map((t) => <ToolRow key={t.id} tool={t} />)}
      </div>
      <Row label={s.ytdlpBuildsRow} note={s.ytdlpBuildsNote}>
        <PopUpButton<ReleaseChannel>
          label={s.ytdlpBuildsLabel}
          value={st.ytdlpChannel}
          onChange={(ytdlpChannel) => change({ ytdlpChannel })}
          choices={[
            { value: "stable", label: s.channelStable },
            { value: "nightly", label: s.channelNightly },
          ]}
        />
      </Row>
      <Row label={s.updatesRow} note={s.updatesNote}>
        <Switch
          label={s.autoUpdateTools}
          checked={st.autoUpdateTools}
          onChange={(autoUpdateTools) => change({ autoUpdateTools })}
        />
      </Row>
      <Row label={s.installedToolsRow}>
        <Switch
          label={s.preferSystemTools}
          checked={st.useSystemTools}
          onChange={(useSystemTools) => change({ useSystemTools })}
        />
      </Row>
      <Row label="">
        <Button onClick={() => void commands.componentsRefresh()}>{s.checkAgain}</Button>
      </Row>
    </>
  );
}

function Advanced({ st }: { st: Settings }) {
  return (
    <>
      <Row label={s.ytdlpRow} note={s.customYtdlpNote}>
        <Field
          key={st.customYtdlp}
          label={s.customYtdlpLabel}
          wide
          value={st.customYtdlp}
          placeholder={s.managedByApp}
          onSave={(customYtdlp) => change({ customYtdlp })}
        />
      </Row>
      <Row label={s.ffmpegFolderRow}>
        <Field
          key={st.customFfmpegDir}
          label={s.customFfmpegLabel}
          wide
          value={st.customFfmpegDir}
          placeholder={s.managedByApp}
          onSave={(customFfmpegDir) => change({ customFfmpegDir })}
        />
      </Row>
      <Row label={s.jsRuntimeRow} note={s.jsRuntimeNote}>
        <Field
          key={st.customJsRuntime}
          label={s.customJsLabel}
          wide
          value={st.customJsRuntime}
          placeholder={s.jsRuntimePlaceholder}
          onSave={(customJsRuntime) => change({ customJsRuntime })}
        />
      </Row>
      <Row label={s.fragmentsRow} note={s.fragmentsNote}>
        <PopUpButton
          label={s.fragmentsLabel}
          value={st.concurrentFragments}
          choices={[1, 2, 4, 8, 16].map((n) => ({ value: n, label: String(n) }))}
          onChange={(concurrentFragments) => change({ concurrentFragments })}
        />
      </Row>
      <div className={styles.separator} />
      <Row label={s.extraArgsRow} note={s.extraArgsNote}>
        <div className={styles.stack}>
          <Switch
            label={s.extraArgsSwitch}
            checked={st.customArgsEnabled}
            onChange={(customArgsEnabled) => change({ customArgsEnabled })}
          />
          <textarea
            className={styles.textarea}
            aria-label={s.extraArgsLabel}
            disabled={!st.customArgsEnabled}
            defaultValue={st.customArgs.join("\n")}
            spellCheck={false}
            onBlur={(e) =>
              change({
                customArgs: e.target.value
                  .split("\n")
                  .map((l) => l.trim())
                  .filter(Boolean),
              })
            }
          />
        </div>
      </Row>
      <div className={styles.separator} />
      <Row label={s.appUpdatesRow}>
        <Switch
          label={s.autoUpdateApp}
          checked={st.autoUpdateApp}
          onChange={(autoUpdateApp) => change({ autoUpdateApp })}
        />
      </Row>
    </>
  );
}

export function App() {
  const [tab, setTab] = useState<Tab>("general");
  const st = useApp((state) => state.settings);
  useConnection();
  return (
    <div className={styles.window}>
      <div className={styles.toolbar} data-tauri-drag-region>
        <div className={styles.lights}>
          <TrafficLights canMinimize={false} canZoom={false} />
        </div>
        <div className={styles.tabs} role="tablist" data-tauri-drag-region>
          {tabs.map((t) => (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={tab === t.id}
              className={styles.tab}
              onClick={() => setTab(t.id)}
            >
              {tab === t.id && (
                <motion.span layoutId="settings-tab" className={styles.tabHighlight} transition={snappy} />
              )}
              <Icon icon={t.icon} size={20} />
              <span>{t.label}</span>
            </button>
          ))}
        </div>
      </div>
      <div className={styles.page} role="tabpanel">
        {st && tab === "general" && <General st={st} />}
        {st && tab === "formats" && <Formats st={st} />}
        {st && tab === "network" && <Network st={st} />}
        {st && tab === "components" && <Components st={st} />}
        {st && tab === "advanced" && <Advanced st={st} />}
      </div>
      <MenuHost />
    </div>
  );
}
