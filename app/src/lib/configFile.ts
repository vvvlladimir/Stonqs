/** JSON files via the webview's own pickers; not a host command, and bytes never become a path. */

/** Offers `text` as a download. The browser owns the dialog, so there is nothing to await. */
export function downloadJson(name: string, text: string): void {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name.endsWith(".json") ? name : `${name}.json`;
  link.click();
  // Revoking immediately can cancel the download in some webviews; a tick is enough.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Opens the system picker and resolves with the file's text, or `null` if nothing was picked. */
export function pickJsonFile(): Promise<string | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    // eslint-disable-next-line lingui/no-unlocalized-strings -- a MIME filter, not text
    input.accept = "application/json,.json";
    input.onchange = async () => {
      const file = input.files?.[0];
      resolve(file ? await file.text() : null);
    };
    // A cancelled picker fires nothing in older webviews, so the promise is left pending
    // rather than resolving wrongly; the dialog is gone and the caller simply does nothing.
    input.click();
  });
}

/** A file name safe on every platform: no separators, no trailing dots. */
export function fileNameOf(label: string, fallback: string): string {
  const clean = label
    .trim()
    .replace(/[^\p{L}\p{N} _-]/gu, "")
    .trim();
  return clean === "" ? fallback : clean;
}
