// What can be done to a job, shared by rows, context menus, keyboard shortcuts and history.

import type { MenuItem } from "../../components/Menu";
import type { Job } from "../../ipc";
import { commands } from "../../ipc";
import { openSheet, useApp } from "../../store/app";
import { isActive } from "../../store/derive";
import { fixLabel, s } from "../../strings";

/** Copies text and briefly tells screen readers so. */
export async function copy(text: Promise<string> | string) {
  await commands.copyText(await text);
  useApp.setState({ announcement: s.copied });
}

export async function applyFix(job: Job) {
  const fix = job.error?.fix;
  if (!fix) return;
  switch (fix) {
    case "useCookies":
      await commands.openSettings();
      return;
    case "chooseFormat":
      openSheet({ kind: "formats", jobId: job.id });
      return;
    case "chooseFolder": {
      // Changes where this download goes and tries again; the default folder stays as it is.
      const current = job.request.destination ?? useApp.getState().settings?.downloadDir ?? "";
      const folder = await commands.chooseFolder(current);
      if (!folder) return;
      await commands.checkFolder(folder);
      await commands.jobUpdateRequest(job.id, { ...job.request, destination: folder });
      return;
    }
    default:
      await commands.jobApplyFix(job.id, fix);
  }
}

export function primaryAction(job: Job): (() => void) | null {
  if (isActive(job.state) || job.state === "queued") return () => void commands.jobPause(job.id);
  if (job.state === "paused") return () => void commands.jobResume(job.id);
  if (job.state === "completed") return () => void commands.openFile(job.id);
  return null;
}

export function jobMenu(job: Job, selection: string[]): MenuItem[] {
  const many = selection.length > 1 && selection.includes(job.id) ? selection : [job.id];
  const link = { label: s.copyLink, onSelect: () => void commands.copyLink(job.id) };
  const details = { label: s.copyDetails, onSelect: () => void copy(commands.jobDetails(job.id)) };
  const remove = { label: s.remove, onSelect: () => void commands.jobRemove(many) };
  switch (job.state) {
    case "probing":
    case "downloading":
    case "postProcessing":
    case "queued":
      return [
        { label: s.pause, onSelect: () => many.forEach((id) => void commands.jobPause(id)) },
        { label: s.cancel, onSelect: () => many.forEach((id) => void commands.jobCancel(id)) },
        { separator: true },
        link,
        details,
      ];
    case "paused":
      return [
        { label: s.resume, onSelect: () => many.forEach((id) => void commands.jobResume(id)) },
        { label: s.cancel, onSelect: () => many.forEach((id) => void commands.jobCancel(id)) },
        { separator: true },
        link,
        remove,
      ];
    case "failed": {
      const fix = job.error?.fix;
      const items: MenuItem[] = [];
      if (fix && fix !== "retry") items.push({ label: fixLabel[fix], onSelect: () => void applyFix(job) });
      if (job.error?.retryable !== false || fix === "retry")
        items.push({ label: s.retry, onSelect: () => void commands.jobRetry(job.id) });
      items.push({ label: s.changeFormat, onSelect: () => openSheet({ kind: "formats", jobId: job.id }) });
      return [...items, { separator: true }, link, details, { separator: true }, remove];
    }
    case "canceled":
      return [
        { label: s.retry, onSelect: () => void commands.jobRetry(job.id) },
        remove,
        { separator: true },
        link,
      ];
    case "completed":
      return [
        { label: s.open, onSelect: () => void commands.openFile(job.id) },
        { label: s.showInExplorer, onSelect: () => void commands.reveal(job.id) },
        { separator: true },
        link,
        { label: s.downloadAgain, onSelect: () => void commands.downloadAgain(job.id) },
        { separator: true },
        remove,
        { label: s.moveToRecycleBin, destructive: true, onSelect: () => void commands.historyTrash(many) },
      ];
  }
}
