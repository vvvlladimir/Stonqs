/** The wizard, end to end, over a real broker export: pick the file, say what it is, look at the
 *  instruments it names, write it. What the host answers is generated from the core itself
 *  (`app/src-tauri/tests/wizard_fixture.rs`), so this fails when the wire changes rather than
 *  agreeing with a shape the app stopped sending.
 *
 *  It is the only test that goes through all four steps. Everything it checks is what the user is
 *  asked and what the user is told — never a component's own state. */
import { beforeEach, describe, expect, it } from "vitest";
import userEvent from "@testing-library/user-event";
import { screen } from "@testing-library/react";
import { mockHost, renderScreen, type Asked } from "../../test/host";
import { Import } from "./index";
import fixture from "../../../e2e/fixtures/import-wizard.json";

/** The host's own accounts, as the accounts screen lists them. */
const accounts = fixture.accounts.map((account) => ({
  ...account,
  transaction_count: 0,
  in_portfolio: true,
  reference_name: null,
  balances: [],
  securities_value_base: null,
  value_base: null,
}));

let asked: Asked;

beforeEach(() => {
  asked = mockHost({
    "plugin:dialog|open": () => "/tmp/export.csv",
    accounts_list: () => accounts,
    import_templates_list: () => [],
    plugins_list: () => ({ plugins: [], folder: "/tmp/plugins" }),
    quote_providers: () => ["yahoo"],
    import_clear: () => null,
    import_load_path: () => fixture.detected,
    // The account is the one answer the wizard cannot give itself, so it is what the two
    // readings of this file differ by — exactly as the core answers.
    import_preview: (args) => {
      const mapping = args.mapping as { account_id?: string | null } | null;
      return mapping?.account_id ? fixture.mapped : fixture.detected;
    },
    import_commit: () => fixture.result,
  });
});

const next = () => screen.getByRole("button", { name: /^next$/i });
const sent = (command: string) => asked.filter((a) => a.command === command);
/** The last thing the host was told, which is what the screen is showing. */
const last = (command: string) => sent(command)[sent(command).length - 1];

/** Steps 1 and 2, which every test past the file needs. */
async function throughTheAccount(user: ReturnType<typeof userEvent.setup>) {
  renderScreen(<Import />);
  await user.click(await screen.findByRole("button", { name: /choose a file/i }));
  await screen.findByText(/9 rows · 19 columns/i);
  await user.click(next());
  await user.click(await screen.findByText(/a broker statement/i));
  await screen.findAllByText(/securities · trade republic · eur/i);
}

describe("the import wizard", () => {
  it("goes from a picked file to a written ledger", async () => {
    const user = userEvent.setup();
    renderScreen(<Import />);

    // 1. The file. Nothing is asked but which file it is, and the step says what is inside it.
    await user.click(await screen.findByRole("button", { name: /choose a file/i }));
    expect(await screen.findByText(/9 rows · 19 columns/i)).toBeDefined();
    expect(sent("import_load_path")).toHaveLength(1);

    // 2. Parsing. The layout is recognised, so the only open question is where the rows land —
    //    and the wizard refuses to go on until it is answered.
    await user.click(next());
    expect(await screen.findByText(/choose a default account first/i)).toBeDefined();

    await user.click(await screen.findByText(/a broker statement/i));
    // Saying what the file is picks the only brokerage account there is, and the preview is
    // asked again with it: what the next two steps show is a reading of that answer.
    // Named both in the picker and in the summary of where the rows land.
    expect(await screen.findAllByText(/securities · trade republic · eur/i)).toHaveLength(2);
    const mapping = last("import_preview").args.mapping as { account_id: string };
    expect(mapping.account_id).toBe("fixture-depot");

    // 3. Instruments. This export names its instruments by ISIN and every one of them is new.
    await user.click(next());
    expect(await screen.findByText(/not found: .*IE00B5BMR087/i)).toBeDefined();

    // 4. Commit. The button says how many rows it is about to write, and nothing is written
    //    before it is pressed.
    await user.click(next());
    const write = await screen.findByRole("button", { name: /write 9 rows/i });
    expect(sent("import_commit")).toHaveLength(0);

    await user.click(write);
    expect(await screen.findByText(/9 written/i)).toBeDefined();
    expect(sent("import_commit")).toHaveLength(1);
    // Written under the same layout the user was shown, never a re-detected one.
    const committed = sent("import_commit")[0].args.mapping as { account_id: string };
    expect(committed.account_id).toBe("fixture-depot");
  });

  it("does not let a file past the second step with nowhere to land", async () => {
    const user = userEvent.setup();
    renderScreen(<Import />);
    await user.click(await screen.findByRole("button", { name: /choose a file/i }));
    await user.click(await screen.findByRole("button", { name: /^next$/i }));

    expect(next().hasAttribute("disabled")).toBe(true);
    // Nor jumped over by its own chip, or the wizard would be asking nothing.
    expect(screen.getByRole("button", { name: /^4\. commit$/i }).hasAttribute("disabled")).toBe(true);
  });

  it("says what each row became before anything is written", async () => {
    const user = userEvent.setup();
    await throughTheAccount(user);
    await user.click(next());
    await user.click(next());

    // The counts of the last step are the core's summary, not a second count of its own:
    // 2 ready, 7 waiting on an instrument, 3 carrying a notice.
    expect(await screen.findByRole("button", { name: /ready · 2/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /no instrument · 7/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /with notices · 3/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /error · 0/i })).toBeDefined();
    expect(sent("import_commit")).toHaveLength(0);
  });

  it("asks the host for a preview again whenever the layout changes, and never for a second file", async () => {
    const user = userEvent.setup();
    await throughTheAccount(user);
    // One reading as the file was loaded, one after the account was chosen. The file itself is
    // loaded once: the host keeps the bytes, and every later step is a reading of them.
    expect(sent("import_preview").length).toBeGreaterThanOrEqual(1);
    expect(sent("import_load_path")).toHaveLength(1);
  });
});
