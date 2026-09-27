import { useLayoutEffect, useRef } from "react";
import { useLingui } from "@lingui/react/macro";
import { PaperPlaneRightIcon, StopIcon } from "@phosphor-icons/react";

/** The input box and the one button beside it, which sends or stops depending on what the turn
 * is doing. The box grows with the text up to a few lines and then scrolls: a conversation is
 * the point of the panel, not the draft. */
export function Composer({
  text,
  busy,
  placeholder,
  onChange,
  onSubmit,
  onStop,
}: {
  text: string;
  busy: boolean;
  placeholder: string;
  onChange: (text: string) => void;
  onSubmit: () => void;
  onStop: () => void;
}) {
  const { t } = useLingui();
  const box = useRef<HTMLTextAreaElement>(null);

  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [text]);

  return (
    <div className="ai-input">
      <textarea
        ref={box}
        rows={1}
        value={text}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            onSubmit();
          }
        }}
      />
      {busy ? (
        <button type="button" className="ai-send ai-send--stop" aria-label={t`Stop`} onClick={onStop}>
          <StopIcon weight="fill" />
        </button>
      ) : (
        <button
          type="button"
          className="ai-send"
          disabled={!text.trim()}
          aria-label={t`Send`}
          onClick={onSubmit}
        >
          <PaperPlaneRightIcon weight="fill" />
        </button>
      )}
    </div>
  );
}
