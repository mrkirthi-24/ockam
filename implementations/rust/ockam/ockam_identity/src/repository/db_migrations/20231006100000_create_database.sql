CREATE TABLE identity (
  identifier TEXT,
  change_history BLOB
);

CREATE TABLE identity_attributes (
  identifier TEXT,
  attributes BLOB
  added INTEGER NOT NULL,
  expires INTEGER,
  attested_by TEXT
);
