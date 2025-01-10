-- Your SQL goes here


ALTER TABLE "users" DROP COLUMN "data";
ALTER TABLE "users" ADD COLUMN "data" INT4[] NOT NULL;

