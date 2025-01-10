-- This file should undo anything in `up.sql`


ALTER TABLE "users" DROP COLUMN "data";
ALTER TABLE "users" ADD COLUMN "data" BYTEA NOT NULL;

