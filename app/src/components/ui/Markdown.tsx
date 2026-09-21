import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

/**
 * Markdown as the app renders it: GitHub flavour (tables, strikethrough, task lists) and
 * **no raw HTML**. `react-markdown` ignores embedded HTML unless `rehype-raw` is added, and it
 * is deliberately not — this renders text that came from a model, which in turn read text that
 * came out of somebody else's CSV.
 *
 * A primitive, so it knows nothing about who wrote the text: the chat uses it for both sides of
 * the conversation, and anything else with authored prose can too.
 */
export function Markdown({ children }: { children: string }) {
  return (
    <div className="md">
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
    </div>
  );
}
