-- Your SQL goes here
DROP TABLE IF EXISTS "pools";
DROP TABLE IF EXISTS "positions";
DROP TABLE IF EXISTS "price_tvls";
CREATE TABLE "prices"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"pool_id" TEXT NOT NULL,
	"price" NUMERIC NOT NULL,
	"tvl" NUMERIC NOT NULL
);

CREATE TABLE "trades"(
	"id" TEXT NOT NULL PRIMARY KEY,
	"decimals" INT2 NOT NULL,
	"token_program_id" TEXT NOT NULL,
	"coin_vault" TEXT NOT NULL,
	"coin_mint" TEXT NOT NULL,
	"pc_vault" TEXT NOT NULL,
	"pc_mint" TEXT NOT NULL,
	"amount" NUMERIC NOT NULL,
	"entry_time" TIMESTAMPTZ,
	"entry_price" NUMERIC,
	"exit_price" NUMERIC,
	"exit_time" TIMESTAMPTZ,
	"pct" NUMERIC NOT NULL,
	"state" INT4 NOT NULL,
	"tx_id" TEXT,
	"sol_before" NUMERIC NOT NULL,
	"sol_after" NUMERIC,
	"trade_kp" BYTEA NOT NULL,
	"root_kp" BYTEA NOT NULL
);

