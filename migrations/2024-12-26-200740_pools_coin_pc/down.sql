-- This file should undo anything in `up.sql`
ALTER TABLE "pools" DROP COLUMN "coin_vault";
ALTER TABLE "pools" DROP COLUMN "coin_mint";
ALTER TABLE "pools" DROP COLUMN "pc_vault";
ALTER TABLE "pools" DROP COLUMN "pc_mint";
ALTER TABLE "pools" ADD COLUMN "sol_vault" TEXT NOT NULL;
ALTER TABLE "pools" ADD COLUMN "vault" TEXT NOT NULL;
ALTER TABLE "pools" ADD COLUMN "mint" TEXT NOT NULL;



