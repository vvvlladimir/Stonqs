-- Whether taking money out gives allowance back. Most allowances do not — an ISA deposit stays
-- spent after it is withdrawn — so every limit, the existing ones included, starts at 0. A
-- "flexible" ISA is the exception and is the user's switch to turn on. See ADR-0071.
ALTER TABLE contribution_limits ADD COLUMN withdrawals_restore INTEGER NOT NULL DEFAULT 0;
