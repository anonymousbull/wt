-- This file should undo anything in `up.sql`
CREATE TABLE "prices"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"price" NUMERIC NOT NULL,
	"tvl" NUMERIC NOT NULL,
	"trade_id" TEXT NOT NULL
);


