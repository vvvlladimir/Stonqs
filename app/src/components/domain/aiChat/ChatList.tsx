/** Every chat, newest first, with rename, export and delete. */

import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { ChatCircleDotsIcon, PlusIcon, SparkleIcon } from "@phosphor-icons/react";
import { api } from "../../../lib/api";
import { formatDateTime } from "../../../lib/format";
import { keys, useAiChats, useAiProviders, useInvalidate } from "../../../lib/queries";
import type { AiChat } from "../../../lib/types";
import {
  Banner,
  ErrorText,
  Field,
  FormDialog,
  ListRow,
  Pending,
  QueryError,
  useMenu,
  type MenuItem,
} from "../../ui";

export function ChatList({ onOpen }: { onOpen: (id: string) => void }) {
  const { t } = useLingui();
  const chats = useAiChats();
  const invalidate = useInvalidate();
  const providers = useAiProviders();
  // A chat carries its own provider, so what blocks a new one is having no key anywhere — not
  // the state of the one a new chat happens to start on.
  const connected = providers.data?.some((provider) => provider.connected) ?? false;
  const menu = useMenu();
  // The chat being renamed, held with its current title so the dialog opens on what it says now.
  const [renaming, setRenaming] = useState<{ id: string; title: string } | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);

  const create = useMutation({
    mutationFn: () => api.aiChatCreate(t`New chat`),
    onSuccess: (chat) => {
      invalidate(keys.aiChats());
      onOpen(chat.id);
    },
  });
  const remove = useMutation({
    mutationFn: (id: string) => api.aiChatDelete(id),
    onSuccess: () => invalidate(keys.aiChats()),
  });
  const rename = useMutation({
    mutationFn: ({ id, title }: { id: string; title: string }) => api.aiChatRename(id, title),
    onSuccess: () => {
      setRenaming(null);
      invalidate(keys.aiChats());
    },
  });

  // Markdown, not CSV: a chat is prose with readings in it, and the panel already renders it as
  // markdown — the file is what is on screen plus the tool calls behind it.
  const exportChat = async (chat: AiChat) => {
    const path = await save({
      defaultPath: `${fileName(chat.title)}.md`,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    if (!path) return;
    setSaveError(null);
    try {
      await api.aiChatExportSave(chat.id, path);
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : String(error));
    }
  };

  if (chats.isError) return <QueryError error={chats.error} />;
  if (!chats.data) return <Pending />;

  // A chat names itself from its first message; renaming by hand wins from then on, which is why
  // the host never retitles a chat that already has history.
  const itemsFor = (chat: AiChat): MenuItem[] => [
    { label: t`Rename…`, onSelect: () => setRenaming({ id: chat.id, title: chat.title }) },
    { label: t`Export as Markdown…`, onSelect: () => void exportChat(chat) },
    { label: t`Delete`, danger: true, onSelect: () => remove.mutate(chat.id) },
  ];

  return (
    <>
      <div className="ai-body ai-body--list">
        {!connected && (
          <Banner tone="info">
            <Trans>Add your API key in Settings → AI assistant to start chatting.</Trans>
          </Banner>
        )}
        {chats.data.length === 0 ? (
          <div className="ai-opener">
            <span className="ai-mark ai-mark--big" aria-hidden>
              <SparkleIcon weight="fill" />
            </span>
            <h3>
              <Trans>No chats yet</Trans>
            </h3>
            <p className="muted">
              <Trans>Ask about performance, allocation, a single holding — anything on screen.</Trans>
            </p>
          </div>
        ) : (
          <div className="ai-chats">
            {chats.data.map((chat: AiChat) => (
              <ListRow
                key={chat.id}
                box
                lead={<ChatCircleDotsIcon />}
                title={chat.title}
                sub={formatDateTime(chat.updated_at)}
                onClick={() => onOpen(chat.id)}
                {...menu.row(chat.id, itemsFor(chat))}
                actions={menu.button(chat.id, itemsFor(chat))}
              />
            ))}
          </div>
        )}
        {saveError && (
          <ErrorText>
            <Trans>Could not save the file: {saveError}</Trans>
          </ErrorText>
        )}
        {menu.node}
        {renaming && (
          <FormDialog
            title={t`Rename chat`}
            onClose={() => setRenaming(null)}
            onSubmit={() => rename.mutate({ id: renaming.id, title: renaming.title.trim() })}
            busy={rename.isPending}
            error={rename.error}
            ready={renaming.title.trim() !== ""}
          >
            <Field label={t`Title`}>
              <input
                autoFocus
                value={renaming.title}
                onChange={(e) => setRenaming({ ...renaming, title: e.target.value })}
              />
            </Field>
          </FormDialog>
        )}
      </div>
      {/* Floating over the list rather than heading it: starting a chat is what this screen is
          for, and the button stays under the thumb however far the history has been scrolled. */}
      <button
        type="button"
        className="ai-fab"
        disabled={create.isPending || !connected}
        onClick={() => create.mutate()}
      >
        <PlusIcon weight="bold" />
        <Trans>New chat</Trans>
      </button>
    </>
  );
}

/** A title is the user's own words and can hold anything; a file name cannot. */
function fileName(title: string): string {
  const cleaned = title.replace(/[\\/:*?"<>|]/g, " ").trim();
  return cleaned === "" ? "chat" : cleaned.slice(0, 60);
}
