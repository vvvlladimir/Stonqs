-- Split brokerage accounts into securities and deposit accounts.
-- New deposit IDs are deterministic so the migration is reproducible.

ALTER TABLE accounts ADD COLUMN reference_account_id TEXT REFERENCES accounts (id) ON DELETE RESTRICT;

-- 1. Create a deposit account for each former brokerage account.
INSERT INTO accounts (id, name, currency, kind, is_active, opened_at, reference_account_id)
SELECT 'cash-' || id, name || ' · cash', currency, 'DEPOSIT', is_active, opened_at, NULL
FROM accounts
WHERE kind = 'BROKERAGE';

-- 2. Add each deposit account to the same portfolios as its securities account.
INSERT INTO portfolio_accounts (portfolio_id, account_id)
SELECT pa.portfolio_id, 'cash-' || pa.account_id
FROM portfolio_accounts pa
JOIN accounts a ON a.id = pa.account_id
WHERE a.kind = 'BROKERAGE';

-- 3. Move cash operations by kind; fees may reference a security but remain cash.
UPDATE transactions
SET account_id = 'cash-' || account_id
WHERE kind NOT IN (
        'BUY', 'SELL', 'DIVIDEND',
        'DELIVERY_INBOUND', 'DELIVERY_OUTBOUND',
        'SECURITY_TRANSFER_IN', 'SECURITY_TRANSFER_OUT'
    )
  AND account_id IN (SELECT id FROM accounts WHERE kind = 'BROKERAGE');

-- 4. Convert the original accounts and link them to their deposits.
UPDATE accounts
SET kind = 'SECURITIES', reference_account_id = 'cash-' || id
WHERE kind = 'BROKERAGE';

UPDATE accounts SET kind = 'DEPOSIT' WHERE kind = 'CASH';

-- Account groups are overlapping views, distinct from portfolio membership.
CREATE TABLE account_groups (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE account_group_members (
    group_id   TEXT NOT NULL REFERENCES account_groups (id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, account_id)
);
