-- Store a theme palette slot instead of HEX; NULL preserves the legacy rank-based color.
ALTER TABLE taxonomy_nodes ADD COLUMN color INTEGER;
