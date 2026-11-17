// Every string the UI shows, in one place so the app can be translated later.
// Sentence case, plain verbs, specific words. Errors say what happened and one fix.

import type { ErrorKind, Fix, Stage } from "./ipc";

export const s = {
  app: "lovelyDL",

  // Toolbar
  urlPlaceholder: "Paste a link, or drop one anywhere",
  urlLabel: "Link to download",
  toggleSidebar: "Show or hide the sidebar",
  more: "More",
  settings: "Settings",
  close: "Close",
  minimize: "Minimize",
  zoom: "Zoom",

  // Sidebar
  downloads: "Downloads",
  history: "History",
  failed: "Failed",

  // More menu
  newDownload: "New download",
  pasteAndDownload: "Paste and download",
  openDownloadFolder: "Open download folder",
  pauseAll: "Pause all",
  resumeAll: "Resume all",
  retryFailed: "Retry failed",
  clearCompleted: "Clear completed",
  supportedSites: "Supported sites",
  keyboardShortcuts: "Keyboard shortcuts",
  reportIssue: "Report an issue",
  checkForUpdates: "Check for updates…",
  about: "About lovelyDL",

  // Empty states
  emptyTitle: "Nothing downloading",
  emptyBody: "Paste a link with Ctrl+V, or drag one here from your browser.",
  emptyFailedTitle: "No failed downloads",
  emptyFailedBody: "Downloads that fail show up here with a way to fix them.",
  emptyHistoryTitle: "No history yet",
  emptyHistoryBody: "Finished downloads are listed here.",
  noMatches: "No matches",

  sections: "Sections",

  // Queue sections
  sectionActive: "Active",
  sectionQueued: "Queued",
  sectionPaused: "Paused",
  sectionFailed: "Failed",
  sectionCanceled: "Canceled",
  sectionCompleted: "Completed",

  // Row actions
  pause: "Pause",
  resume: "Resume",
  cancel: "Cancel",
  retry: "Retry",
  remove: "Remove from list",
  showInExplorer: "Show in Explorer",
  open: "Open",
  copyLink: "Copy link",
  copyDetails: "Copy details",
  downloadAgain: "Download again",
  removeFromHistory: "Remove from history",
  moveToRecycleBin: "Move to Recycle Bin",
  changeFormat: "Change format…",
  copied: "Copied",

  // Row status
  waiting: "Waiting",
  paused: "Paused",
  canceled: "Canceled",
  fileMissing: "File moved or deleted",
  ofTotal: (done: string, total: string) => `${done} of ${total}`,
  stageOf: (i: number, n: number) => `${i} of ${n}`,

  // Preview
  gettingDetails: "Getting details…",
  download: "Download",
  downloadCount: (n: number) => (n === 1 ? "Download 1 video" : `Download ${n} videos`),
  dismissPreview: "Close preview",
  formatLabel: "Format",
  qualityLabel: "Quality",
  options: "Options",
  advanced: "Advanced…",
  bestQuality: "Best quality",
  compatibleMp4: "Compatible MP4",
  anyHeight: "Highest available",
  audioOnly: "Audio only",
  sourceTopsOut: (h: number) => `This video is ${h}p at most`,
  presetTopsOut: (h: number, src: number) => `${h}p in this format; the source has ${src}p`,
  approx: "~",
  live: "Live",
  upcoming: "Upcoming",
  ageRestricted: "18+",
  membersOnly: "Members only",
  drm: "DRM",
  drmBody: "This video is protected by DRM. lovelyDL does not download DRM-protected media.",
  ambiguousTitle: "This link is a video in a playlist",
  ambiguousBody: "Download just this video, or choose from the whole playlist?",
  justThisVideo: "Just this video",
  wholePlaylist: "Choose from playlist…",
  destination: "Save to",
  changeDestination: "Change…",

  // Playlist sheet
  playlistTitle: (title: string) => title || "Playlist",
  playlistCount: (n: number, loading: boolean) =>
    loading ? `${n} found so far…` : n === 1 ? "1 video" : `${n} videos`,
  searchPlaylist: "Search this playlist",
  selectAll: "Select all",
  selectNone: "Select none",
  skipDownloaded: "Skip items already downloaded",
  alreadyDownloaded: "Downloaded",
  cancelButton: "Cancel",

  // Quit
  quitTitle: (n: number) => (n === 1 ? "Quit and pause 1 download?" : `Quit and pause ${n} downloads?`),
  quitBody: "They continue where they left off the next time you open lovelyDL.",
  quitButton: "Quit",

  // Clipboard
  clipboardOffer: "You copied a link.",
  clipboardAdd: "Add it",
  clipboardDismiss: "Not now",

  // Drop
  dropHint: "Drop to add",

  // History
  searchHistory: "Search history",
  filterAll: "All",
  filterVideo: "Video",
  filterAudio: "Audio",
  filterFailed: "Failed",

  // Errors from adding links
  noLink: "No link found. Paste a web address that starts with http or https.",

  // Legal line, shown in onboarding and the README
  legal: "Download only what you have the right to save.",

  // Common buttons and tool states
  done: "Done",
  back: "Back",
  continueButton: "Continue",
  later: "Later",
  copy: "Copy",
  install: "Install",
  installing: "Installing…",
  installingTool: (name: string) => `Installing ${name}`,
  installAndRestart: "Install and restart",
  update: "Update",
  checking: "Checking…",
  notInstalled: "Not installed",
  cantSaveFolder: (error: string) => `lovelyDL can't save to that folder: ${error}`,

  // Keyboard shortcuts sheet
  shortcutAddCopied: "Add the copied link",
  shortcutDownloadPreview: "Download the previewed link",
  shortcutPauseResume: "Pause or resume the selected downloads",
  shortcutMoveQueued: "Move a waiting download up or down the queue",
  shortcutRemove: "Remove the selected downloads",
  shortcutSelectAll: "Select all downloads",
  shortcutSections: "Downloads, History, Failed",

  // About sheet
  aboutVersion: (version: string) => `Version ${version}. Free software under the GPL, version 3 or later.`,
  github: "GitHub",

  // Updates sheet and banner
  upToDate: "lovelyDL is up to date.",
  updateFound: (version: string) =>
    `lovelyDL ${version} is available. Downloads pause and continue after the restart.`,
  updateBannerLabel: (version: string) => `lovelyDL ${version} is available`,
  updateBannerText: (version: string) => `lovelyDL ${version} is available.`,

  // Formats sheet
  formatsTitle: "Formats",
  formatsBody: "Pick a video stream and an audio stream; ffmpeg joins them.",
  formatsVideo: "Video",
  formatsAudio: "Audio",
  formatAudioCell: "audio",
  chosenByPreset: "Chosen by the preset",
  useAndRetry: "Use and retry",
  useThese: "Use these",

  // Subtitles and sponsor segments (Options sheet and Settings)
  subtitlesRow: "Subtitles:",
  downloadSubtitles: "Download subtitles",
  downloadSubtitlesWith: (languages: string) => `Download subtitles (available: ${languages})`,
  languages: "Languages",
  subtitleLanguages: "Subtitle languages",
  autoCaptions: "Include auto-generated captions",
  sponsorMark: "Mark as chapters",
  sponsorRemove: "Cut out",

  // Options sheet
  optionsTitle: "Options for this download",
  saveToRow: "Save to:",
  useDefault: "Use default",
  partRow: "Only part of it:",
  startTime: "Start time",
  startPlaceholder: "Start, e.g. 1:30",
  timeTo: "to",
  endTime: "End time",
  endPlaceholder: "End, e.g. 2:45",
  timeFormatError: "Use seconds or minutes:seconds, like 90 or 1:30.",
  preciseCuts: "Cut exactly at these times (slower, re-encodes the edges)",
  sponsorSegmentsRow: "Sponsor segments:",
  sponsorSegmentsLabel: "Sponsor segments",
  sponsorKeep: "Keep as they are",
  liveRow: "Live stream:",
  liveFromStart: "Record from the start of the stream",
  waitForLive: "If it has not started, wait for it",
  useOptions: "Use these options",

  // Onboarding
  toolPurposeYtdlp: "Reads the site and downloads the media",
  toolPurposeFfmpeg: "Joins video and audio, converts formats",
  toolPurposeDeno: "Lets yt-dlp read YouTube",
  toolFound: (version: string) => `Found, ${version}`,
  toolBroken: "Doesn't run",
  welcomeTitle: "Welcome to lovelyDL",
  welcomeLead:
    "Paste a link, get the file. Video and audio from the sites yt-dlp supports, saved to your PC.",
  folderTitle: "Where should downloads go?",
  folderLead: "You can change this later, or for a single download.",
  toolsTitle: "Get the tools",
  toolsLead:
    "lovelyDL uses three free tools. It downloads them from their official releases, checks each file, and keeps them up to date.",
  useSystemTools: "Use tools already installed on this PC",
  doneTitle: "You're set",
  doneLead: "Copy a link in your browser, come back, and press Ctrl+V. Or drag the link onto the window.",
  startUsing: "Start using lovelyDL",
  setUpLater: "Set up later",
  toolsLater: "Downloads need the tools: install them from Settings › Components when you are ready.",

  // Settings window
  tabGeneral: "General",
  tabFormats: "Formats",
  tabNetwork: "Network & Accounts",
  tabComponents: "Components",
  tabAdvanced: "Advanced",

  // Settings: General
  saveDownloadsTo: "Save downloads to:",
  onAddRow: "When a link is added:",
  onAddLabel: "When a link is added",
  onAddPreview: "Show the preview",
  onAddDownload: "Download straight away",
  concurrentRow: "Downloads at once:",
  concurrentNote: "How many run at the same time, and from one site.",
  concurrentLabel: "Downloads at once",
  perSite: "per site",
  perSiteLabel: "Downloads per site",
  notificationsRow: "Notifications:",
  notificationsSwitch: "Tell me when downloads finish or fail",
  clipboardRow: "Clipboard:",
  clipboardNote:
    "When you switch to lovelyDL, it checks whether you copied a link and offers to add it. It never reads the clipboard in the background.",
  clipboardSwitch: "Notice copied links",
  keepAwakeRow: "While downloading:",
  keepAwakeSwitch: "Keep the computer awake",
  trayRow: "System tray:",
  trayNote: "Shows a small queue in the notification area. Closing the window then keeps lovelyDL running.",
  traySwitch: "Show lovelyDL in the system tray",

  // Settings: Formats
  defaultFormatRow: "Default format:",
  embedRow: "Embed:",
  embedThumbnail: "Thumbnail as cover art",
  embedMetadata: "Title, artist and other metadata",
  embedChapters: "Chapters",
  subLangsPlaceholder: "en, fr",
  subtitleFilesLabel: "Subtitle files",
  embedSubs: "Embed in the video",
  separateSubs: "Save as separate files",
  sponsorBlockRow: "SponsorBlock:",
  sponsorBlockNote: "Segments come from the SponsorBlock community database.",
  sponsorBlockLabel: "SponsorBlock",
  sponsorOff: "Off",
  fileNamesRow: "File names:",
  fileNameExample: "Example:",
  fileNameTemplate: "File name template",
  playlistsRow: "Playlists:",
  playlistIndex: "Number the files in playlist order",

  // Settings: Network & Accounts
  speedLimitRow: "Speed limit:",
  speedLimitNote: "For example 2M for 2 MB/s, or 500K. Empty means no limit.",
  speedLimitLabel: "Speed limit",
  noLimit: "No limit",
  proxyRow: "Proxy:",
  proxyNote: "http://, https:// or socks5:// with a host and port.",
  proxyLabel: "Proxy",
  templateNotSaved: "Not saved: yt-dlp cannot use this template. The example below says why.",
  proxyInvalid: "Not saved: that is not a proxy address. Downloads still go direct.",
  proxyNone: "None",
  bigPlaylistsRow: "Big playlists:",
  paceNote: "Waits a few seconds between requests, so sites are less likely to block a long run.",
  paceSwitch: "Pace requests",
  cookiesRow: "Signed-in content:",
  cookiesNoteFirefox:
    "yt-dlp reads Firefox's cookies at download time. lovelyDL never copies or stores them.",
  cookiesNoteChromium: (browser: string) =>
    `Close ${browser} before downloading: Windows locks its cookies while it runs. lovelyDL never copies or stores them.`,
  cookiesNoteNone:
    "Use a browser where you are signed in, for videos that need an account. Cookies are read at download time and never stored.",
  cookiesLabel: "Use cookies from",
  cookiesFrom: (browser: string) => `Use cookies from ${browser}`,
  noCookies: "Don't use cookies",
  bookmarkletRow: "Bookmarklet:",
  bookmarkletNote:
    "Make a bookmark with this as its address. Clicking it on a video page opens the page in lovelyDL.",

  // Settings: Components
  ffmpegAndFfprobe: "ffmpeg and ffprobe",
  originManaged: "kept up to date by lovelyDL",
  originSystem: "installed on this PC",
  originCustom: "your own copy",
  toolBrokenDetail: (message: string) => `Doesn't run: ${message}`,
  versionAvailable: (version: string) => `${version} available`,
  previousVersion: "Previous version",
  ytdlpBuildsRow: "yt-dlp builds:",
  ytdlpBuildsNote: "Nightly builds fix broken sites sooner, and are tested less.",
  ytdlpBuildsLabel: "yt-dlp builds",
  channelStable: "Stable",
  channelNightly: "Nightly",
  updatesRow: "Updates:",
  updatesNote: "Checked at most once a day, and never while something is downloading.",
  autoUpdateTools: "Update the tools automatically",
  installedToolsRow: "Installed tools:",
  preferSystemTools: "Prefer tools already installed on this PC",
  checkAgain: "Check again",

  // Settings: Advanced
  ytdlpRow: "yt-dlp:",
  customYtdlpNote: "A yt-dlp.exe to use instead of lovelyDL's own.",
  customYtdlpLabel: "Custom yt-dlp",
  managedByApp: "Managed by lovelyDL",
  ffmpegFolderRow: "ffmpeg folder:",
  customFfmpegLabel: "Custom ffmpeg folder",
  jsRuntimeRow: "JavaScript runtime:",
  jsRuntimeNote:
    "name:path, for example node:C:\\Program Files\\nodejs\\node.exe. Deno, Node 22+, QuickJS and Bun work.",
  customJsLabel: "Custom JavaScript runtime",
  jsRuntimePlaceholder: "Deno, managed by lovelyDL",
  fragmentsRow: "Fragments at once:",
  fragmentsNote: "Speeds up streams that arrive in pieces. Some sites limit this.",
  fragmentsLabel: "Fragments at once",
  extraArgsRow: "Extra yt-dlp options:",
  extraArgsNote:
    "One option per line. For people who know yt-dlp; links and imported settings can never set these.",
  extraArgsSwitch: "Pass extra options to yt-dlp",
  extraArgsLabel: "Extra yt-dlp options",
  appUpdatesRow: "lovelyDL updates:",
  autoUpdateApp: "Check for updates automatically",
} as const;

// SponsorBlock category ids and their names, in the order Settings lists them.
export const sponsorCategoryLabel: Record<string, string> = {
  sponsor: "Sponsors",
  selfpromo: "Self-promotion",
  interaction: "Subscribe reminders",
  intro: "Intros",
  outro: "Endcards",
  preview: "Previews and recaps",
  music_offtopic: "Non-music in music videos",
  filler: "Filler",
};

export const stageLabel: Record<Stage, string> = {
  waiting: "Waiting",
  probing: "Getting details",
  preparing: "Preparing",
  video: "Downloading video",
  audio: "Downloading audio",
  media: "Downloading",
  subtitles: "Downloading subtitles",
  merging: "Merging video and audio",
  extractingAudio: "Extracting audio",
  converting: "Converting",
  embeddingMetadata: "Embedding metadata",
  embeddingThumbnail: "Embedding thumbnail",
  embeddingSubtitles: "Embedding subtitles",
  sponsorBlock: "Handling sponsor segments",
  fixing: "Finishing",
  moving: "Saving",
  waitingForLive: "Waiting for the stream to start",
  done: "Done",
};

export const errorCopy: Record<ErrorKind, { title: string; body: string }> = {
  signIn: {
    title: "Sign-in required",
    body: "The site wants a signed-in visitor before it shows this video.",
  },
  ageRestricted: { title: "Age-restricted", body: "The site only shows this video to signed-in adults." },
  private: { title: "Private video", body: "Only people the owner invited can see this video." },
  unavailable: { title: "Video unavailable", body: "It was removed, made private, or never existed." },
  geoBlocked: { title: "Not available in your region", body: "The site blocks this video where you are." },
  membersOnly: { title: "Members only", body: "This video is for the channel's paying members." },
  drm: { title: "Protected by DRM", body: "lovelyDL does not download DRM-protected media." },
  unsupportedUrl: { title: "Site not supported", body: "yt-dlp does not know how to read this page." },
  formatUnavailable: {
    title: "Format not available",
    body: "The video is not offered in the format you picked.",
  },
  http403: {
    title: "Access refused",
    body: "The site refused the download. A newer yt-dlp often fixes this.",
  },
  http429: {
    title: "Too many requests",
    body: "The site is limiting downloads from your connection for a while.",
  },
  jsRuntime: { title: "Deno is missing", body: "yt-dlp needs Deno to read this site." },
  ffmpegMissing: { title: "ffmpeg is missing", body: "ffmpeg is needed to merge and convert downloads." },
  ytdlpMissing: { title: "yt-dlp is missing", body: "lovelyDL needs yt-dlp to download anything." },
  diskFull: { title: "Disk full", body: "There is not enough space left in the download folder." },
  permissionDenied: {
    title: "Can't save here",
    body: "Windows did not allow lovelyDL to write to this folder.",
  },
  network: { title: "No connection", body: "The site could not be reached." },
  cookies: {
    title: "Couldn't read cookies",
    body: "The browser's cookies could not be read. Close the browser and retry.",
  },
  liveNotStarted: { title: "Not live yet", body: "This stream has not started." },
  playlist: { title: "This is a playlist", body: "Choose which videos to download." },
  crashed: { title: "yt-dlp stopped", body: "yt-dlp quit without saying why." },
  unknown: { title: "Download failed", body: "yt-dlp reported a problem." },
};

/** The explanation for an error: ours, or yt-dlp's own words when we have nothing better. */
export function errorBody(kind: ErrorKind, detail: string): string {
  return (kind === "unknown" || kind === "crashed") && detail ? detail : errorCopy[kind].body;
}

export const fixLabel: Record<Fix, string> = {
  retry: "Retry",
  useCookies: "Use cookies from a browser…",
  chooseFormat: "Choose another format…",
  updateYtdlp: "Update yt-dlp and retry",
  installComponents: "Install missing components",
  chooseFolder: "Choose another folder…",
  waitForLive: "Wait for the stream",
  chooseVideos: "Choose videos…",
};
