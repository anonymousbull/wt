-- This file should undo anything in `up.sql`

ALTER TABLE "positions" DROP COLUMN "trade_kp";
ALTER TABLE "positions" DROP COLUMN "root_kp";
ALTER TABLE "positions" ADD COLUMN "kp" BYTEA NOT NULL;


