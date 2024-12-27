-- Your SQL goes here

ALTER TABLE "positions" DROP COLUMN "kp";
ALTER TABLE "positions" ADD COLUMN "trade_kp" BYTEA NOT NULL;
ALTER TABLE "positions" ADD COLUMN "root_kp" BYTEA NOT NULL;


