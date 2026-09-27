import { Suspense, lazy } from "react";

const MarkdownBody = lazy(() => import("./MarkdownBody"));

/** GitHub-flavoured markdown with **no raw HTML** (no `rehype-raw`): the text came from a model
 * that read somebody's CSV. The renderer loads lazily; until then the text shows as is. */
export function Markdown({ children }: { children: string }) {
  return (
    <div className="md">
      <Suspense fallback={<p>{children}</p>}>
        <MarkdownBody>{children}</MarkdownBody>
      </Suspense>
    </div>
  );
}
