import dotenv from "dotenv";
import { Connection, Keypair } from "@solana/web3.js";
import { PumpFunSDK } from "pumpdotfun-sdk";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { AnchorProvider } from "@coral-xyz/anchor";

dotenv.config();

const getProvider = () => {
    if (!process.env.SOLANA_RPC_URL2) {
        throw new Error("Please set HELIUS_RPC_URL in .env file");
    }

    const connection = new Connection(process.env.SOLANA_RPC_URL2, {commitment:"processed"});
    const wallet = new NodeWallet(new Keypair());
    return new AnchorProvider(connection, wallet, { commitment: "processed" });
};

const setupEventListeners = async (sdk) => {
    // const createEventId = sdk.addEventListener("createEvent", (event, slot, signature) => {
    //     console.log("createEvent", event, slot, signature);
    // });
    // console.log("Subscribed to createEvent with ID:", createEventId);
    //
    // const tradeEventId = sdk.addEventListener("tradeEvent", (event, slot, signature) => {
    //     console.log("tradeEvent", event, slot, signature);
    // });
    // console.log("Subscribed to tradeEvent with ID:", tradeEventId);

    const completeEventId = sdk.addEventListener("completeEvent", (event, slot, signature) => {
        console.log("completeEvent", event, slot, signature);
    });
    console.log("Subscribed to completeEvent with ID:", completeEventId);
};

const main = async () => {
    try {
        const provider = getProvider();
        const sdk = new PumpFunSDK(provider);

        // Set up event listeners
        await setupEventListeners(sdk);
    } catch (error) {
        console.error("An error occurred:", error);
    }
};

main();