-- A commission is taken on the venue and withholding is deducted where the issuer sits, so
-- either can be charged in a currency the operation itself is not settled in. NULL means "the
-- transaction's own currency", which is what every existing row is.
ALTER TABLE transactions ADD COLUMN fee_currency TEXT;
ALTER TABLE transactions ADD COLUMN tax_currency TEXT;
