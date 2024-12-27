-- This file should undo anything in `up.sql`
ALTER TABLE "prices" DROP COLUMN "trade_id";
ALTER TABLE "prices" ADD COLUMN "pool_id" TEXT NOT NULL;


