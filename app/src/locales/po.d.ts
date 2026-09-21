/** `@lingui/vite-plugin` compiles an imported `.po` file into a message catalog. */
declare module "*.po" {
  import type { Messages } from "@lingui/core";

  export const messages: Messages;
}
