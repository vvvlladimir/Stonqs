import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { CheckCircleIcon } from "@phosphor-icons/react";

/** Toasts confirm completed actions; persistent errors use banners or field text. */
const ToastContext = createContext<(message: string) => void>(() => {});

export function useToast() {
  return useContext(ToastContext);
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [message, setMessage] = useState<{ text: string; at: number } | null>(null);
  const show = useCallback((text: string) => setMessage({ text, at: Date.now() }), []);

  useEffect(() => {
    if (!message) return;
    const timer = window.setTimeout(() => setMessage(null), 3000);
    return () => window.clearTimeout(timer);
  }, [message]);

  const value = useMemo(() => show, [show]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      {message &&
        createPortal(
          <div className="toast" role="status" key={message.at}>
            <CheckCircleIcon weight="fill" />
            {message.text}
          </div>,
          document.body,
        )}
    </ToastContext.Provider>
  );
}
