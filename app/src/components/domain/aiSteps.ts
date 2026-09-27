/** Steps built from live events and from stored blocks, kept out of `AiTools.tsx` so it exports components only. */

import type { AiBlock } from "../../lib/types";
import type { LiveTool } from "../../lib/ai";

/** A reading, a web search or a reasoning summary; one shape live and stored. */
export interface Step {
  key: string;
  kind: "tool" | "search" | "thinking";
  /** The tool's name, or the search query. Unused by `thinking`. */
  name: string;
  /** The model's own line on why it asked. Under the row, never part of its title. */
  why?: string;
  sent?: string;
  answered?: string;
  /** Prose rather than a call and its answer — what the model thought, not what it asked for. */
  text?: string;
  busy?: boolean;
}

/** The stored blocks of one finished turn as steps, results joined to the calls that asked. */
export function storedSteps(blocks: AiBlock[], results: Map<string, AiBlock>, turn: string): Step[] {
  const steps: Step[] = [];
  blocks.forEach((block, i) => {
    if (block.type === "reasoning") {
      steps.push({ key: `${turn}:r${i}`, kind: "thinking", name: "", text: block.text });
    } else if (block.type === "web_search") {
      steps.push({ key: `${turn}:s${i}`, kind: "search", name: block.query });
    } else if (block.type === "tool_call") {
      const result = results.get(block.id);
      steps.push({
        key: `${turn}:${block.id}`,
        kind: "tool",
        name: block.name,
        sent: block.args_json,
        answered: result?.type === "tool_result" ? result.content : undefined,
      });
    }
  });
  return steps;
}

/** The same steps while the turn is still running. */
export function liveSteps(live: LiveTool[], thinking: string, pending: string): Step[] {
  const steps: Step[] = [];
  if (thinking) {
    // Still being written until the answer itself starts arriving.
    steps.push({ key: "live:think", kind: "thinking", name: "", text: thinking, busy: !pending });
  }
  live.forEach((entry, i) => {
    steps.push({
      key: `live:${i}`,
      kind: entry.kind === "search" ? "search" : "tool",
      name: entry.name,
      why: entry.reason,
      answered: entry.content,
      busy: entry.content === undefined,
    });
  });
  return steps;
}
