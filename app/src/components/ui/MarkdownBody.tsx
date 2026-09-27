import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

/** The heavy renderer in its own module, loaded on demand. */
export default function MarkdownBody({ children }: { children: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        // Every link leaves the app, so it opens where links from this app open, and never
        // in place — a navigation would drop the whole session.
        a: ({ children, ...props }) => (
          <a {...props} target="_blank" rel="noreferrer noopener">
            {children}
          </a>
        ),
      }}
    >
      {children}
    </ReactMarkdown>
  );
}
