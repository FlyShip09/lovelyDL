// Window-wide input: Ctrl+V anywhere adds the link, links dragged from a browser are accepted,
// and the keyboard shortcuts work from wherever focus is.

import { useEffect } from "react";
import { create } from "zustand";

import { commands } from "../../ipc";
import { addLinks, openSheet, setView, useApp } from "../../store/app";
import { toggleSidebar } from "./Toolbar";

/** Whether a link is being dragged over the window, for the drop highlight. */
export const useDragging = create<{ over: boolean }>()(() => ({ over: false }));

function editing(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest("input, textarea, [contenteditable]") !== null;
}

function linkTypes(e: DragEvent): boolean {
  const types = e.dataTransfer?.types ?? [];
  return types.includes("text/uri-list") || types.includes("text/plain");
}

export function useGlobalInput() {
  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      if (editing(e.target) || useApp.getState().sheet) return;
      const text = e.clipboardData?.getData("text/plain") ?? "";
      if (!text) return;
      e.preventDefault();
      void addLinks(text, "paste");
    };

    // Drag events fire for every child; counting enters and leaves keeps the highlight steady.
    let depth = 0;
    const onEnter = (e: DragEvent) => {
      if (!linkTypes(e)) return;
      e.preventDefault();
      depth += 1;
      useDragging.setState({ over: true });
    };
    const onOver = (e: DragEvent) => {
      if (!linkTypes(e)) return;
      e.preventDefault();
      if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
    };
    const onLeave = () => {
      depth = Math.max(0, depth - 1);
      if (depth === 0) useDragging.setState({ over: false });
    };
    const onDrop = (e: DragEvent) => {
      depth = 0;
      useDragging.setState({ over: false });
      if (!linkTypes(e)) return;
      e.preventDefault();
      // Browsers put the link in text/uri-list (one per line, # lines are comments).
      const list = (e.dataTransfer?.getData("text/uri-list") ?? "")
        .split(/\r?\n/)
        .filter((l) => l && !l.startsWith("#"))
        .join("\n");
      const text = list || (e.dataTransfer?.getData("text/plain") ?? "");
      if (text) void addLinks(text, "drop");
    };

    const onKey = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey && !e.altKey && !e.metaKey;
      const key = e.key.toLowerCase();
      if (ctrl && !e.shiftKey && key === "n") {
        setView("downloads");
        document.querySelector<HTMLInputElement>("[data-url-field]")?.focus();
      } else if (ctrl && e.shiftKey && key === "v") {
        void commands.clipboardTakeLink().catch(() => undefined);
      } else if (ctrl && key === ",") {
        void commands.openSettings();
      } else if (ctrl && key === "/") {
        openSheet({ kind: "shortcuts" });
      } else if (e.ctrlKey && e.altKey && key === "s") {
        toggleSidebar();
      } else if (ctrl && (key === "1" || key === "2" || key === "3")) {
        setView(key === "1" ? "downloads" : key === "2" ? "history" : "failed");
      } else if (ctrl && key === "f" && useApp.getState().view === "history") {
        document.querySelector<HTMLInputElement>("[data-history-search]")?.focus();
      } else if (ctrl && key === "q") {
        void commands.windowAction("close");
      } else if (e.key === "F11") {
        void commands.windowAction("zoom");
      } else {
        return;
      }
      e.preventDefault();
    };

    document.addEventListener("paste", onPaste);
    window.addEventListener("dragenter", onEnter);
    window.addEventListener("dragover", onOver);
    window.addEventListener("dragleave", onLeave);
    window.addEventListener("drop", onDrop);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("paste", onPaste);
      window.removeEventListener("dragenter", onEnter);
      window.removeEventListener("dragover", onOver);
      window.removeEventListener("dragleave", onLeave);
      window.removeEventListener("drop", onDrop);
      window.removeEventListener("keydown", onKey);
    };
  }, []);
}
