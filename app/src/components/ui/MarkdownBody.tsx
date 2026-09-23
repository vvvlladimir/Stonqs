import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

/**
 * The renderer itself, in a module of its own so `Markdown` can load it on demand: the parser
 * and its GitHub-flavour extensions are the heaviest dependency in the app, and nothing on the
 * first screen renders prose.
 */
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
