-- Your SQL goes here
CREATE TABLE "positions"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"amount" NUMERIC NOT NULL,
	"entry_time" TIMESTAMPTZ,
	"pool" TEXT NOT NULL,
	"entry_price" NUMERIC,
	"exit_price" NUMERIC,
	"exit_time" TIMESTAMPTZ,
	"pct" NUMERIC NOT NULL,
	"state" INT4 NOT NULL,
	"tx_id" TEXT,
	"sol_before" NUMERIC NOT NULL,
	"sol_after" NUMERIC,
	"kp" BYTEA NOT NULL
);

CREATE TABLE "price_tvls"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"pool_id" TEXT NOT NULL,
	"price" NUMERIC NOT NULL,
	"tvl" NUMERIC NOT NULL
);

CREATE TABLE "pools"(
	"id" TEXT NOT NULL PRIMARY KEY,
	"sol_vault" TEXT NOT NULL,
	"vault" TEXT NOT NULL,
	"mint" TEXT NOT NULL,
	"decimals" INT2 NOT NULL,
	"token_program_id" TEXT NOT NULL
);

