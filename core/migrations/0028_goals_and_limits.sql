-- Savings goals and contribution limits. See ADR-0068.

-- A goal names its own accounts, so it does not follow the picker; no rows in `goal_accounts`
-- means the whole portfolio. `expected_return` is the user's assumption, `0` meaning pure
-- saving — it is never read from what the portfolio actually returned.
CREATE TABLE goals (
    id              TEXT PRIMARY KEY,
    portfolio_id    TEXT NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    target_amount   TEXT NOT NULL,
    currency        TEXT NOT NULL,
    -- Absent asks the other question: when would this pace arrive.
    target_date     TEXT,
    monthly_amount  TEXT,
    expected_return TEXT NOT NULL DEFAULT '0',
    note            TEXT,
    created_at      TEXT NOT NULL
);
CREATE INDEX idx_goals_portfolio ON goals (portfolio_id);

CREATE TABLE goal_accounts (
    goal_id    TEXT NOT NULL REFERENCES goals (id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    PRIMARY KEY (goal_id, account_id)
);

-- What one account may take in one limit year. `year_starts_on` is `MM-DD` because the year a
-- ceiling is administered over is not always the calendar one — the UK's begins on 6 April.
-- No country and no shipped ceiling: the rules differ per year as well as per country.
CREATE TABLE contribution_limits (
    id             TEXT PRIMARY KEY,
    account_id     TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    name           TEXT NOT NULL,
    amount         TEXT NOT NULL,
    currency       TEXT NOT NULL,
    year_starts_on TEXT NOT NULL DEFAULT '01-01',
    note           TEXT
);
CREATE INDEX idx_contribution_limits_account ON contribution_limits (account_id);
