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
} as const;

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
