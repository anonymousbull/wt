-- Your SQL goes here

CREATE TABLE "trades"(
	"id" TEXT NOT NULL PRIMARY KEY,
	"coin_vault" TEXT NOT NULL,
	"pc_vault" TEXT NOT NULL,
	"coin_mint" TEXT NOT NULL,
	"pc_mint" TEXT NOT NULL,
	"decimals" INT2 NOT NULL,
	"token_program_id" TEXT NOT NULL,
	"amount" NUMERIC NOT NULL,
	"entry_time" TIMESTAMPTZ,
	"entry_price" NUMERIC,
	"exit_time" TIMESTAMPTZ,
	"exit_price" NUMERIC,
	"pct" NUMERIC NOT NULL,
	"state" INT4 NOT NULL,
	"tx_in_id" TEXT,
	"tx_out_id" TEXT,
	"sol_before" NUMERIC NOT NULL,
	"sol_after" NUMERIC,
	"root_kp" BYTEA NOT NULL
);

