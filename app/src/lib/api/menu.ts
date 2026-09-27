/** The native menu bar, rebuilt only when its shape changes. */

import { CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu } from "@tauri-apps/api/menu";

/** A native item whose behaviour belongs to the OS: copy and paste, hide, quit. */
export type NativeMenuKind =
  | "About"
  | "Services"
  | "Hide"
  | "HideOthers"
  | "ShowAll"
  | "Quit"
  | "Undo"
  | "Redo"
  | "Cut"
  | "Copy"
  | "Paste"
  | "SelectAll"
  | "Minimize"
  | "Maximize"
  | "Fullscreen"
  | "CloseWindow"
  | "BringAllToFront";

/** An item that can be picked; `checked` present makes it a check item. */
export interface AppMenuItem {
  id: string;
  text: string;
  accelerator?: string;
  enabled?: boolean;
  checked?: boolean;
}

export interface AppMenuSubmenu {
  id: string;
  submenu: string;
  items: AppMenuEntry[];
  enabled?: boolean;
}

export type AppMenuEntry =
  "separator" | AppMenuItem | AppMenuSubmenu | { native: NativeMenuKind; text: string };

export interface AppMenuSection {
  text: string;
  items: AppMenuEntry[];
}

type Handle = MenuItem | CheckMenuItem | Submenu;

/** What the menu bar was last built from: its shape, and a handle for every item that can change. */
let built: { shape: string; handles: Map<string, Handle>; state: Map<string, string> } | null = null;
let queue: Promise<void> = Promise.resolve();

/** The spec without what changes in place, so a toggled check or a disabled item is not a rebuild. */
function shapeOf(sections: AppMenuSection[]): string {
  return JSON.stringify(sections, (key, value) =>
    key === "enabled" ? undefined : key === "checked" ? true : value,
  );
}

function stateOf(entry: AppMenuItem | AppMenuSubmenu): string {
  return `${entry.enabled !== false}|${"checked" in entry ? entry.checked : ""}`;
}

/** Every text is already translated; a same-shaped spec only updates `enabled`/`checked`. Calls are serialised. */
export function setAppMenu(sections: AppMenuSection[], onPick: (id: string) => void): Promise<void> {
  // A failed build must not wedge every later one behind it.
  queue = queue.catch(() => {}).then(() => applyMenu(sections, onPick));
  return queue;
}

async function applyMenu(sections: AppMenuSection[], onPick: (id: string) => void): Promise<void> {
  const shape = shapeOf(sections);
  if (built && built.shape === shape) {
    const updates: Promise<void>[] = [];
    const walk = (entries: AppMenuEntry[]) => {
      for (const entry of entries) {
        if (entry === "separator" || "native" in entry) continue;
        const state = stateOf(entry);
        const handle = built!.handles.get(entry.id);
        if (handle && built!.state.get(entry.id) !== state) {
          built!.state.set(entry.id, state);
          updates.push(handle.setEnabled(entry.enabled !== false));
          if ("checked" in entry && handle instanceof CheckMenuItem)
            updates.push(handle.setChecked(!!entry.checked));
        }
        if ("submenu" in entry) walk(entry.items);
      }
    };
    sections.forEach((section) => walk(section.items));
    await Promise.all(updates);
    return;
  }

  const handles = new Map<string, Handle>();
  const state = new Map<string, string>();
  const build = async (entry: AppMenuEntry): Promise<Handle | PredefinedMenuItem> => {
    // eslint-disable-next-line lingui/no-unlocalized-strings -- a native item kind
    if (entry === "separator") return PredefinedMenuItem.new({ item: "Separator" });
    if ("native" in entry) {
      const kind = entry.native === "About" ? { About: null } : entry.native;
      return PredefinedMenuItem.new({ item: kind, text: entry.text });
    }
    let handle: Handle;
    if ("submenu" in entry) {
      const items = await Promise.all(entry.items.map(build));
      handle = await Submenu.new({
        id: entry.id,
        text: entry.submenu,
        enabled: entry.enabled !== false,
        items,
      });
    } else if ("checked" in entry) {
      const { checked, ...rest } = entry;
      handle = await CheckMenuItem.new({ ...rest, checked: !!checked, action: onPick });
    } else {
      handle = await MenuItem.new({ ...entry, action: onPick });
    }
    handles.set(entry.id, handle);
    state.set(entry.id, stateOf(entry));
    return handle;
  };
  const submenus = await Promise.all(
    sections.map(async (section) =>
      Submenu.new({ text: section.text, items: await Promise.all(section.items.map(build)) }),
    ),
  );
  const menu = await Menu.new({ items: submenus });
  await menu.setAsAppMenu();
  built = { shape, handles, state };
}
