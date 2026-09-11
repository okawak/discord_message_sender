import type { MessageInstruction } from "../pkg/parse_message.js";

export function isSavedClippingInstruction(
  messageId: string,
  instruction: MessageInstruction,
  existingClippingIds: ReadonlySet<string>,
): boolean {
  return instruction.kind === "url" && existingClippingIds.has(messageId);
}
