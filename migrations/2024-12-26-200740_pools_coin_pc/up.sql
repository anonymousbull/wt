-- Your SQL goes here
ALTER TABLE "pools" DROP COLUMN "sol_vault";
ALTER TABLE "pools" DROP COLUMN "vault";
ALTER TABLE "pools" DROP COLUMN "mint";
ALTER TABLE "pools" ADD COLUMN "coin_vault" TEXT NOT NULL;
ALTER TABLE "pools" ADD COLUMN "coin_mint" TEXT NOT NULL;
ALTER TABLE "pools" ADD COLUMN "pc_vault" TEXT NOT NULL;
ALTER TABLE "pools" ADD COLUMN "pc_mint" TEXT NOT NULL;



