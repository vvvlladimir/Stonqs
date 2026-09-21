-- Seed asset-class, region, and sector trees so allocation is useful on first launch.
-- Fixed IDs make this one-time data migration deterministic; cash is intentionally excluded.
-- Names are user data: English here, renamed by the user, never translated by the UI.

INSERT INTO taxonomies (id, name, kind) VALUES
    ('tax-asset-class', 'Asset class', 'ASSET_CLASS'),
    ('tax-region',      'Region',      'REGION'),
    ('tax-sector',      'Sector',      'SECTOR');

INSERT INTO taxonomy_nodes (id, taxonomy_id, parent_id, name, rank, color) VALUES
    -- Asset class
    ('tn-ac-equity',    'tax-asset-class', NULL,            'Equities',              0, 1),
    ('tn-ac-eq-dev',    'tax-asset-class', 'tn-ac-equity',  'Developed markets',     0, 1),
    ('tn-ac-eq-em',     'tax-asset-class', 'tn-ac-equity',  'Emerging markets',      1, 2),
    ('tn-ac-eq-single', 'tax-asset-class', 'tn-ac-equity',  'Single companies',      2, 3),
    ('tn-ac-bonds',     'tax-asset-class', NULL,            'Bonds',                 1, 3),
    ('tn-ac-bond-gov',  'tax-asset-class', 'tn-ac-bonds',   'Government',            0, 3),
    ('tn-ac-bond-corp', 'tax-asset-class', 'tn-ac-bonds',   'Corporate',             1, 4),
    ('tn-ac-commod',    'tax-asset-class', NULL,            'Commodities',           2, 5),
    ('tn-ac-realestate','tax-asset-class', NULL,            'Real estate',           3, 6),
    ('tn-ac-crypto',    'tax-asset-class', NULL,            'Crypto',                4, 7),

    -- Region
    ('tn-rg-na',        'tax-region', NULL,         'North America',                 0, 1),
    ('tn-rg-us',        'tax-region', 'tn-rg-na',   'United States',                 0, 1),
    ('tn-rg-ca',        'tax-region', 'tn-rg-na',   'Canada',                        1, 2),
    ('tn-rg-eu',        'tax-region', NULL,         'Europe',                        1, 2),
    ('tn-rg-ez',        'tax-region', 'tn-rg-eu',   'Eurozone',                      0, 2),
    ('tn-rg-eu-ex',     'tax-region', 'tn-rg-eu',   'Europe ex-euro',                1, 3),
    ('tn-rg-jp',        'tax-region', NULL,         'Japan',                         2, 3),
    ('tn-rg-apac',      'tax-region', NULL,         'Asia-Pacific ex-Japan',         3, 4),
    ('tn-rg-em',        'tax-region', NULL,         'Emerging markets',              4, 5),
    ('tn-rg-em-asia',   'tax-region', 'tn-rg-em',   'Asia',                          0, 5),
    ('tn-rg-em-other',  'tax-region', 'tn-rg-em',   'Other',                         1, 6),

    -- Sector
    ('tn-sc-tech',      'tax-sector', NULL,          'Technology',                   0, 1),
    ('tn-sc-semis',     'tax-sector', 'tn-sc-tech',  'Semiconductors',               0, 1),
    ('tn-sc-software',  'tax-sector', 'tn-sc-tech',  'Software',                     1, 2),
    ('tn-sc-hardware',  'tax-sector', 'tn-sc-tech',  'Hardware',                     2, 3),
    ('tn-sc-finance',   'tax-sector', NULL,          'Financials',                   1, 2),
    ('tn-sc-health',    'tax-sector', NULL,          'Health care',                  2, 3),
    ('tn-sc-consumer',  'tax-sector', NULL,          'Consumer',                     3, 4),
    ('tn-sc-industry',  'tax-sector', NULL,          'Industrials',                  4, 5),
    ('tn-sc-energy',    'tax-sector', NULL,          'Energy',                       5, 6),
    ('tn-sc-materials', 'tax-sector', NULL,          'Materials',                    6, 7),
    ('tn-sc-utilities', 'tax-sector', NULL,          'Utilities',                    7, 8),
    ('tn-sc-other',     'tax-sector', NULL,          'Other',                        8, 8);
