-- Your SQL goes here

CREATE TABLE "trade_prices"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"trade_id" TEXT NOT NULL,
	"price" NUMERIC NOT NULL,
	"tvl" NUMERIC NOT NULL
);

