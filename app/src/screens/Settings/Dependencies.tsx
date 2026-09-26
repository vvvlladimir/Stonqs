import { useMemo, useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { CaretLeftIcon, CaretRightIcon } from "@phosphor-icons/react";
import notices from "../../generated/notices.json";
import { DataTable, SearchBox, type Column, type SortState } from "../../components/ui";

interface Dependency {
  name: string;
  version: string;
  licence: string;
  url: string;
  /** Which half of the app it is part of; the wording is the user's, not the toolchain's. */
  part: "app" | "interface";
}

const ALL: Dependency[] = [
  ...notices.rust.map((d) => ({ ...d, part: "app" as const })),
  ...notices.js.map((d) => ({ ...d, part: "interface" as const })),
];

/** A list nobody reads to the end: one page of it, and a search for the rest. */
const PAGE = 30;

/** What each column orders by. Paging sorts the whole list before it cuts a page out of it —
 *  ordering the page instead would order thirty rows against each other and nothing else. */
const BY: Record<string, (d: Dependency) => string> = {
  name: (d) => d.name.toLowerCase(),
  version: (d) => d.version,
  part: (d) => d.part,
  licence: (d) => d.licence,
};

/**
 * The dependency table of `AboutPanel`, in its own chunk and rendered only once asked for: the
 * generated list is eight hundred rows, which is a thing to search rather than a page to scroll.
 */
export default function Dependencies() {
  const { t } = useLingui();
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SortState | null>({ key: "name", dir: "asc" });
  const [page, setPage] = useState(0);

  const rows = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const found = needle
      ? ALL.filter((d) => d.name.toLowerCase().includes(needle) || d.licence.toLowerCase().includes(needle))
      : ALL;
    const by = BY[sort?.key ?? "name"] ?? BY.name;
    const dir = sort?.dir === "desc" ? -1 : 1;
    return [...found].sort((a, b) => by(a).localeCompare(by(b)) * dir);
  }, [query, sort]);

  const pages = Math.max(1, Math.ceil(rows.length / PAGE));
  const at = Math.min(page, pages - 1);
  const first = at * PAGE;
  const shown = rows.slice(first, first + PAGE);

  const parts: Record<Dependency["part"], string> = {
    app: t`App`,
    interface: t`Interface`,
  };

  const columns: Column<Dependency>[] = [
    {
      key: "name",
      header: t`Component`,
      align: "left",
      sort: (d) => d.name.toLowerCase(),
      cell: (d) => (
        <a href={d.url} target="_blank" rel="noreferrer noopener">
          {d.name}
        </a>
      ),
    },
    {
      key: "version",
      header: t`Version`,
      width: "9rem",
      align: "left",
      sort: (d) => d.version,
      cell: (d) => <span className="num">{d.version}</span>,
    },
    {
      key: "part",
      header: t`Part`,
      width: "8rem",
      align: "left",
      only: "wide",
      sort: (d) => d.part,
      cell: (d) => parts[d.part],
    },
    {
      key: "licence",
      header: t`Licence`,
      width: "16rem",
      align: "left",
      sort: (d) => d.licence,
      cell: (d) => d.licence,
    },
  ];

  return (
    <>
      <SearchBox
        value={query}
        onChange={(value) => {
          setQuery(value);
          setPage(0);
        }}
        placeholder={t`Search by name or licence`}
      />
      <DataTable
        columns={columns}
        rows={shown}
        rowKey={(d) => `${d.part}:${d.name}@${d.version}`}
        fixed
        sort={sort}
        onSortChange={(next) => {
          setSort(next);
          setPage(0);
        }}
        empty={<Trans>Nothing under that name or licence.</Trans>}
      />
      {rows.length > 0 && (
        <div className="inline">
          <button
            type="button"
            className="btn btn--sm btn--ghost"
            disabled={at === 0}
            onClick={() => setPage(at - 1)}
          >
            <CaretLeftIcon />
            <Trans>Previous</Trans>
          </button>
          <button
            type="button"
            className="btn btn--sm btn--ghost"
            disabled={at >= pages - 1}
            onClick={() => setPage(at + 1)}
          >
            <Trans>Next</Trans>
            <CaretRightIcon />
          </button>
          <span className="dim">
            <Trans>
              {first + 1}–{first + shown.length} of {rows.length}
            </Trans>
          </span>
        </div>
      )}
      <p className="dim">
        <Trans>Full licence texts are in the saved notices.</Trans>
      </p>
    </>
  );
}
