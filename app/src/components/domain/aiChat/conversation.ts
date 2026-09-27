import type { AiBlock, ChatMessage } from "../../../lib/types";
import { storedSteps, type Step } from "../aiSteps";

/** What the panel draws, in order: said things, and runs of steps between them. */
export type Part =
  { kind: "said"; key: string; role: string; text: string } | { kind: "steps"; key: string; steps: Step[] };

/** Stored turns flattened, readings grouped across turns; results join their call by `call_id`. */
export function conversation(messages: ChatMessage[]): Part[] {
  const results = resultsById(messages);
  const parts: Part[] = [];

  for (const message of messages) {
    const steps = storedSteps(message.blocks, results, message.id);
    if (steps.length > 0) {
      const last = parts[parts.length - 1];
      if (last?.kind === "steps") last.steps = [...last.steps, ...steps];
      else parts.push({ kind: "steps", key: `steps:${message.id}`, steps });
    }
    const text = message.blocks
      .filter((block) => block.type === "text")
      .map((block) => (block.type === "text" ? block.text : ""))
      .join("\n\n")
      .trim();
    if (text) parts.push({ kind: "said", key: `said:${message.id}`, role: message.role, text });
  }
  return parts;
}

function resultsById(messages: ChatMessage[]): Map<string, AiBlock> {
  const found = new Map<string, AiBlock>();
  for (const message of messages) {
    for (const block of message.blocks) {
      if (block.type === "tool_result") found.set(block.call_id, block);
    }
  }
  return found;
}
