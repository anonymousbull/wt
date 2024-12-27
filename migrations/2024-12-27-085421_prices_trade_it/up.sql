-- Your SQL goes here
ALTER TABLE "prices" DROP COLUMN "pool_id";
ALTER TABLE "prices" ADD COLUMN "trade_id" TEXT NOT NULL;


