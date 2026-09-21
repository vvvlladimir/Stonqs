-- Regular savings plans: a schedule, an amount, and the split it buys. See ADR-0033.

-- `account_id` is the securities account for a plan with legs and the deposit account for a
-- cash contribution plan; the settlement side is derived from the account, never stored twice.
CREATE TABLE investment_plans (
    id           TEXT PRIMARY KEY,
    portfolio_id TEXT NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    account_id   TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    amount       TEXT NOT NULL,
    currency     TEXT NOT NULL,
    fees         TEXT NOT NULL DEFAULT '0',
    taxes        TEXT NOT NULL DEFAULT '0',
    -- Recurrence: every `interval_count` units of `interval_unit`, counted from `start_date`.
    start_date     TEXT NOT NULL,
    end_date       TEXT,
    interval_unit  TEXT NOT NULL,
    interval_count INTEGER NOT NULL,
    active       INTEGER NOT NULL DEFAULT 1,
    note         TEXT
);
CREATE INDEX idx_investment_plans_portfolio ON investment_plans (portfolio_id);

-- No legs at all means a cash contribution plan. Weights are shares of `amount` and sum to one.
CREATE TABLE plan_legs (
    plan_id     TEXT NOT NULL REFERENCES investment_plans (id) ON DELETE CASCADE,
    security_id TEXT NOT NULL REFERENCES securities (id) ON DELETE CASCADE,
    weight      TEXT NOT NULL,
    position    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (plan_id, security_id)
);

-- Which occurrence produced which transaction. "Last executed" is read from here rather than
-- stored on the plan, so deleting the transaction offers the occurrence again by itself.
CREATE TABLE plan_executions (
    plan_id         TEXT NOT NULL REFERENCES investment_plans (id) ON DELETE CASCADE,
    occurrence_date TEXT NOT NULL,
    transaction_id  TEXT NOT NULL REFERENCES transactions (id) ON DELETE CASCADE,
    PRIMARY KEY (plan_id, occurrence_date, transaction_id)
);
CREATE INDEX idx_plan_executions_transaction ON plan_executions (transaction_id);
