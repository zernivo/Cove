import { invoke } from "@tauri-apps/api/core";

interface Status {
  version: string;
  journal_entries: number;
}

async function loadStatus(): Promise<void> {
  const el = document.getElementById("status");
  if (!el) return;
  try {
    const s = await invoke<Status>("cove_status");
    el.textContent = `Cove ${s.version} · ${s.journal_entries} recorded changes`;
  } catch {
    el.textContent = "Running outside Tauri (browser preview).";
  }
}

loadStatus();
