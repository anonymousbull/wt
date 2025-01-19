use schemars::schema_for;
use wolf_trader::trade::*;

fn main() {
    let schema = schema_for!(BuyPrompt);
    let schema = serde_json::to_string(&schema).unwrap();
    std::fs::write("prompts/buy.json",schema).unwrap();
}