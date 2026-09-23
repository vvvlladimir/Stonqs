import { Suspense, lazy } from "react";

const MarkdownBody = lazy(() => import("./MarkdownBody"));

/**
 * Markdown as the app renders it: GitHub flavour (tables, strikethrough, task lists) and
 * **no raw HTML**. `react-markdown` ignores embedded HTML unless `rehype-raw` is added, and it
 * is deliberately not — this renders text that came from a model, which in turn read text that
 * came out of somebody else's CSV.
 *
 * A primitive, so it knows nothing about who wrote the text: the chat uses it for both sides of
 * the conversation, and anything else with authored prose can too. The renderer arrives with the
 * first prose on screen rather than with the app; until it does the text is shown as it is,
 * because unrendered markdown reads, while a spinner in place of a paragraph does not.
 */
export function Markdown({ children }: { children: string }) {
  return (
    <div className="md">
      <Suspense fallback={<p>{children}</p>}>
        <MarkdownBody>{children}</MarkdownBody>
      </Suspense>
    </div>
  );
}
