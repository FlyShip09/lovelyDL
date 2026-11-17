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
import { connect, saveSettings, useApp } from "../../store/app";
import { PresetControls } from "../downloads/PreviewCard";
import styles from "./settings.module.css";

type Tab = "general" | "formats" | "network" | "components" | "advanced";

const tabs: { id: Tab; label: string; icon: LucideIcon }[] = [
  { id: "general", label: "General", icon: SlidersHorizontal },
  { id: "formats", label: "Formats", icon: Film },
  { id: "network", label: "Network & Accounts", icon: Globe },
  { id: "components", label: "Components", icon: Package },
  { id: "advanced", label: "Advanced", icon: Download },
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
      <Row label="Save downloads to:">
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
          Change…
        </Button>
      </Row>
      <Row label="When a link is added:">
        <PopUpButton<OnAdd>
          label="When a link is added"
          value={st.onAdd}
          onChange={(onAdd) => change({ onAdd })}
          choices={[
            { value: "preview", label: "Show the preview" },
            { value: "download", label: "Download straight away" },
          ]}
        />
      </Row>
      <Row label="Downloads at once:" note="How many run at the same time, and from one site.">
        <PopUpButton
          label="Downloads at once"
          value={st.maxConcurrent}
          choices={counts}
          onChange={(maxConcurrent) => change({ maxConcurrent })}
        />
        <span className={styles.inline}>per site</span>
        <PopUpButton
          label="Downloads per site"
          value={st.maxPerSite}
          choices={counts}
          onChange={(maxPerSite) => change({ maxPerSite })}
        />
      </Row>
      <div className={styles.separator} />
      <Row label="Notifications:">
        <Switch
          label="Tell me when downloads finish or fail"
          checked={st.notifications}
          onChange={(notifications) => change({ notifications })}
        />
      </Row>
      <Row
        label="Clipboard:"
        note="When you switch to lovelyDL, it checks whether you copied a link and offers to add it. It never reads the clipboard in the background."
      >
        <Switch
          label="Notice copied links"
          checked={st.noticeCopiedLinks}
          onChange={(noticeCopiedLinks) => change({ noticeCopiedLinks })}
        />
      </Row>
      <Row label="While downloading:">
        <Switch
          label="Keep the computer awake"
          checked={st.keepAwake}
          onChange={(keepAwake) => change({ keepAwake })}
        />
      </Row>
      <Row
        label="System tray:"
        note="Shows a small queue in the notification area. Closing the window then keeps lovelyDL running."
      >
        <Switch
          label="Show lovelyDL in the system tray"
          checked={st.trayIcon}
          onChange={(trayIcon) => change({ trayIcon })}
        />
      </Row>
    </>
  );
}

const sponsorCategories: [string, string][] = [
  ["sponsor", "Sponsors"],
  ["selfpromo", "Self-promotion"],
  ["interaction", "Subscribe reminders"],
  ["intro", "Intros"],
  ["outro", "Endcards"],
  ["preview", "Previews and recaps"],
  ["music_offtopic", "Non-music in music videos"],
  ["filler", "Filler"],
];

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
      <Row label="Default format:">
        <PresetControls request={req} onChange={(defaultRequest) => change({ defaultRequest })} />
      </Row>
      <Row label="Embed:">
        <div className={styles.stack}>
          <Checkbox
            label="Thumbnail as cover art"
            checked={o.embedThumbnail}
            onChange={(embedThumbnail) => setOptions({ embedThumbnail })}
          />
          <Checkbox
            label="Title, artist and other metadata"
            checked={o.embedMetadata}
            onChange={(embedMetadata) => setOptions({ embedMetadata })}
          />
          <Checkbox
            label="Chapters"
            checked={o.embedChapters}
            onChange={(embedChapters) => setOptions({ embedChapters })}
          />
        </div>
      </Row>
      <Row label="Subtitles:">
        <div className={styles.stack}>
          <Checkbox
            label="Download subtitles"
            checked={o.subtitles}
            onChange={(subtitles) => setOptions({ subtitles })}
          />
          <span className={styles.inlineRow}>
            <span className={styles.inline}>Languages</span>
            <Field
              key={o.subLangs.join(", ")}
              label="Subtitle languages"
              value={o.subLangs.join(", ")}
              placeholder="en, fr"
              onSave={(v) => setOptions({ subLangs: v.split(/[,\s]+/).filter(Boolean) })}
            />
          </span>
          <Checkbox
            label="Include auto-generated captions"
            checked={o.autoSubs}
            onChange={(autoSubs) => setOptions({ autoSubs })}
          />
          <PopUpButton
            label="Subtitle files"
            value={o.embedSubs}
            onChange={(embedSubs) => setOptions({ embedSubs })}
            choices={[
              { value: true, label: "Embed in the video" },
              { value: false, label: "Save as separate files" },
            ]}
          />
        </div>
      </Row>
      <Row label="SponsorBlock:" note="Segments come from the SponsorBlock community database.">
        <div className={styles.stack}>
          <PopUpButton<SponsorBlock>
            label="SponsorBlock"
            value={o.sponsorblock}
            onChange={(sponsorblock) => setOptions({ sponsorblock })}
            choices={[
              { value: "off", label: "Off" },
              { value: "mark", label: "Mark as chapters" },
              { value: "remove", label: "Cut out" },
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
        label="File names:"
        note={
          <>
            Example: <span className={`${styles.mono} selectable`}>{preview}</span>
          </>
        }
      >
        <Field
          key={st.filenameTemplate}
          label="File name template"
          wide
          value={st.filenameTemplate}
          onSave={(filenameTemplate) => change({ filenameTemplate })}
        />
      </Row>
      <Row label="Playlists:">
        <div className={styles.stack}>
          <Checkbox
            label="Number the files in playlist order"
            checked={st.playlistIndex}
            onChange={(playlistIndex) => change({ playlistIndex })}
          />
          <Checkbox
            label="Skip items already downloaded"
            checked={st.skipDownloaded}
            onChange={(skipDownloaded) => change({ skipDownloaded })}
          />
        </div>
      </Row>
    </>
  );
}

const browsers = ["", "firefox", "chrome", "edge", "brave", "opera", "vivaldi", "chromium", "whale"];
const browserName = (b: string) => (b ? (b[0]?.toUpperCase() ?? "") + b.slice(1) : "Don't use cookies");
const bookmarklet = "javascript:location.href='lovelydl://download?url='+encodeURIComponent(location.href)";

function Network({ st }: { st: Settings }) {
  const chromium = ["chrome", "edge", "brave", "opera", "vivaldi", "chromium", "whale"].includes(
    st.cookiesBrowser,
  );
  return (
    <>
      <Row label="Speed limit:" note="For example 2M for 2 MB/s, or 500K. Empty means no limit.">
        <Field
          key={st.rateLimit}
          label="Speed limit"
          value={st.rateLimit}
          placeholder="No limit"
          onSave={(rateLimit) => change({ rateLimit })}
        />
      </Row>
      <Row label="Proxy:" note="http://, https:// or socks5:// with a host and port.">
        <Field
          key={st.proxy}
          label="Proxy"
          wide
          value={st.proxy}
          placeholder="None"
          onSave={(proxy) => change({ proxy })}
        />
      </Row>
      <Row
        label="Big playlists:"
        note="Waits a few seconds between requests, so sites are less likely to block a long run."
      >
        <Switch
          label="Pace requests"
          checked={st.paceRequests}
          onChange={(paceRequests) => change({ paceRequests })}
        />
      </Row>
      <div className={styles.separator} />
      <Row
        label="Signed-in content:"
        note={
          st.cookiesBrowser === "firefox"
            ? "yt-dlp reads Firefox's cookies at download time. lovelyDL never copies or stores them."
            : chromium
              ? `Close ${browserName(st.cookiesBrowser)} before downloading: Windows locks its cookies while it runs. lovelyDL never copies or stores them.`
              : "Use a browser where you are signed in, for videos that need an account. Cookies are read at download time and never stored."
        }
      >
        <PopUpButton
          label="Use cookies from"
          value={st.cookiesBrowser}
          onChange={(cookiesBrowser) => change({ cookiesBrowser })}
          choices={browsers.map((b) => ({
            value: b,
            label: b ? `Use cookies from ${browserName(b)}` : browserName(b),
          }))}
        />
      </Row>
      <Row
        label="Bookmarklet:"
        note="Make a bookmark with this as its address. Clicking it on a video page opens the page in lovelyDL."
      >
        <span className={`${styles.mono} ${styles.path} selectable`}>{bookmarklet}</span>
        <Button onClick={() => void commands.copyText(bookmarklet)}>Copy</Button>
      </Row>
    </>
  );
}

const toolInfo: Record<Tool["id"], string> = { ytdlp: "yt-dlp", ffmpeg: "ffmpeg and ffprobe", deno: "Deno" };

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
            `${tool.version} · ${tool.origin === "managed" ? "kept up to date by lovelyDL" : tool.origin === "system" ? "installed on this PC" : "your own copy"}`}
          {status.status === "missing" && "Not installed"}
          {status.status === "checking" && "Checking…"}
          {status.status === "broken" && `Doesn't run: ${status.message}`}
          {tool.latest && status.status === "ready" && ` · ${tool.latest} available`}
        </span>
        {error && <span className={styles.error}>{error}</span>}
        {status.status === "installing" && (
          <ProgressBar value={status.fraction} label={`Installing ${toolInfo[tool.id]}`} />
        )}
      </div>
      {status.status !== "installing" && (tool.origin === "managed" || status.status !== "ready") && (
        <>
          {status.status === "ready" && (
            <Button onClick={() => void run(commands.componentRollback(tool.id))}>Previous version</Button>
          )}
          <Button onClick={() => void run(commands.componentInstall(tool.id))}>
            {status.status === "ready" ? "Update" : "Install"}
          </Button>
        </>
      )}
    </div>
  );
}

function Components({ st }: { st: Settings }) {
  const components = useApp((s) => s.components);
  return (
    <>
      <div className={styles.tools}>
        {components &&
          [components.ytdlp, components.ffmpeg, components.deno].map((t) => <ToolRow key={t.id} tool={t} />)}
      </div>
      <Row label="yt-dlp builds:" note="Nightly builds fix broken sites sooner, and are tested less.">
        <PopUpButton<ReleaseChannel>
          label="yt-dlp builds"
          value={st.ytdlpChannel}
          onChange={(ytdlpChannel) => change({ ytdlpChannel })}
          choices={[
            { value: "stable", label: "Stable" },
            { value: "nightly", label: "Nightly" },
          ]}
        />
      </Row>
      <Row label="Updates:" note="Checked at most once a day, and never while something is downloading.">
        <Switch
          label="Update the tools automatically"
          checked={st.autoUpdateTools}
          onChange={(autoUpdateTools) => change({ autoUpdateTools })}
        />
      </Row>
      <Row label="Installed tools:">
        <Switch
          label="Prefer tools already installed on this PC"
          checked={st.useSystemTools}
          onChange={(useSystemTools) => change({ useSystemTools })}
        />
      </Row>
      <Row label="">
        <Button onClick={() => void commands.componentsRefresh()}>Check again</Button>
      </Row>
    </>
  );
}

function Advanced({ st }: { st: Settings }) {
  return (
    <>
      <Row label="yt-dlp:" note="A yt-dlp.exe to use instead of lovelyDL's own.">
        <Field
          key={st.customYtdlp}
          label="Custom yt-dlp"
          wide
          value={st.customYtdlp}
          placeholder="Managed by lovelyDL"
          onSave={(customYtdlp) => change({ customYtdlp })}
        />
      </Row>
      <Row label="ffmpeg folder:">
        <Field
          key={st.customFfmpegDir}
          label="Custom ffmpeg folder"
          wide
          value={st.customFfmpegDir}
          placeholder="Managed by lovelyDL"
          onSave={(customFfmpegDir) => change({ customFfmpegDir })}
        />
      </Row>
      <Row
        label="JavaScript runtime:"
        note="name:path, for example node:C:\Program Files\nodejs\node.exe. Deno, Node 22+, QuickJS and Bun work."
      >
        <Field
          key={st.customJsRuntime}
          label="Custom JavaScript runtime"
          wide
          value={st.customJsRuntime}
          placeholder="Deno, managed by lovelyDL"
          onSave={(customJsRuntime) => change({ customJsRuntime })}
        />
      </Row>
      <Row label="Fragments at once:" note="Speeds up streams that arrive in pieces. Some sites limit this.">
        <PopUpButton
          label="Fragments at once"
          value={st.concurrentFragments}
          choices={[1, 2, 4, 8, 16].map((n) => ({ value: n, label: String(n) }))}
          onChange={(concurrentFragments) => change({ concurrentFragments })}
        />
      </Row>
      <div className={styles.separator} />
      <Row
        label="Extra yt-dlp options:"
        note="One option per line. For people who know yt-dlp; links and imported settings can never set these."
      >
        <div className={styles.stack}>
          <Switch
            label="Pass extra options to yt-dlp"
            checked={st.customArgsEnabled}
            onChange={(customArgsEnabled) => change({ customArgsEnabled })}
          />
          <textarea
            className={styles.textarea}
            aria-label="Extra yt-dlp options"
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
      <Row label="lovelyDL updates:">
        <Switch
          label="Check for updates automatically"
          checked={st.autoUpdateApp}
          onChange={(autoUpdateApp) => change({ autoUpdateApp })}
        />
      </Row>
    </>
  );
}

export function App() {
  const [tab, setTab] = useState<Tab>("general");
  const st = useApp((s) => s.settings);
  useEffect(() => {
    let stop: (() => void) | undefined;
    void connect().then((u) => (stop = u));
    return () => stop?.();
  }, []);
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
