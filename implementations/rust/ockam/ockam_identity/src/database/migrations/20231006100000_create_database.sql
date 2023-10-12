CREATE TABLE identity (
  identifier TEXT NOT NULL UNIQUE,
  change_history BLOB NOT NULL,
  name TEXT UNIQUE,
  is_default INTEGER
);

CREATE TABLE identity_enrollment (
  identifier TEXT NOT NULL UNIQUE,
  enrolled_at INTEGER NOT NULL
);

CREATE TABLE identity_attributes (
  identifier TEXT PRIMARY KEY,
  attributes BLOB NOT NULL,
  added INTEGER NOT NULL,
  expires INTEGER,
  attested_by TEXT
);

CREATE TABLE purpose_key (
  identifier TEXT NOT NULL,
  purpose TEXT NOT NULL,
  purpose_key_attestation BLOB NOT NULL
);

CREATE INDEX purpose_key_index ON purpose_key (identifier, purpose);

CREATE TABLE policy (
  resource TEXT NOT NULL,
  action TEXT NOT NULL,
  expression BLOB NOT NULL
);

CREATE TABLE tcp_outlet (
  socket_addr TEXT NOT NULL,
  worker_addr TEXT NOT NULL,
  alias TEXT NOT NULL,
  payload TEXT
);
