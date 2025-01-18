set dotenv-required
set dotenv-load := true

schema:
    cargo r -r --bin schema
user-api port:
    PORT={{port}} cargo r -r --bin user_api
run:
    cargo r -r --bin wolf_trader
test name='' package='':
    cargo test {{name}} {{package}} -- --nocapture
pump:
    cd pump && bun start
diesel:
    diesel setup
dmg name:
    diesel migration generate --diff-schema {{name}}
dmr:
    diesel migration run
dmre:
    diesel migration revert
install:
    cargo install sqlx-cli --no-default-features --features sqlite,postgres
create-db:
    sqlx database create
drop-db:
    sqlx database drop
create-migration env name:
    DATABASE_URL=$PG_URL_{{env}} sqlx migrate add -r {{name}}
migrate env:
    DATABASE_URL=$PG_URL_{{env}} sqlx migrate run
migrate-revert env:
    DATABASE_URL=$PG_URL_{{env}} sqlx migrate revert
stop name:
    pm2 stop {{name}}
restart name:
    pm2 restart {{name}}
serve env:
    DATABASE_UsRL=$PG_URL_{{env}} ENV={{env}} RUST_BACKTRACE=1 pm2 start --name rust -x "cargo" --interpreter none -- r -r --bin femimarket
user-serve:
    cargo r -r --bin user_server
users-lambda:
    cd services/users-lambda && cargo lambda build -r && cargo lambda deploy users-lambda --enable-function-url --profile AdministratorAccess-829290902012 --region eu-west-2
shuttle-users:
    cd services/users-api && shuttle deploy
trade:
    cargo r -r -p trade-engine
rust-ws:
    ENABLE_WEBSOCKET=y RUSTFLAGS="--cfg tokio_unstable" RUST_BACKTRACE=1 cargo r -r --bin wolf_trader
tradea action coin:
    ENABLE_WEBSOCKET=n RUSTFLAGS="--cfg tokio_unstable" RUST_BACKTRACE=1 cargo r -r --bin trade {{action}} {{coin}}
fix:
    RUSTFLAGS="--cfg tokio_unstable" RUST_BACKTRACE=1 cargo fix
flame env:
    RUST_BACKTRACE=1 ENV={{env}} DATABASE_URL=$PG_URL_{{env}} CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --root='--preserve-env' --bin femimarket
runa name:
    cargo r --bin {{name}}
export: test wasm
deno:
    cd serve && pm2 start --name deno -x "deno" --interpreter none -- run --allow-net --allow-env main.ts
solana:
    cd solana && RUST_LOG=info cargo r -r
#test:
#    cargo t -r -- --nocapture && rm -rf webapp/lib/bindings && mv bindings webapp/lib/bindings
#test1:
#    RUSTFLAGS="--cfg tokio_unstable" RUST_BACKTRACE=1 cargo r -r --bin test_trade
add-server-deps:
    cd webserver && bun add @clerk/express
add-ui-deps:
    cd ui && bun add socket.io-client @clerk/clerk-react superjson bignumber.js
add-ui-dev-deps:
    cd ui && bun add -D tailwindcss postcss autoprefixer @types/node && bunx --bun tailwindcss init -p
add-ui-components:
    cd ui && bunx --bun shadcn@latest add button card input label switch tabs select dialog table avatar collapsible separator chart badge tooltip progress alert skeleton
gen-keys:
    echo "Encryption Key" && cargo r --bin gen_enc_key && echo "Signing Key" && cargo r --bin gen_sign_key
serve-ui:
    cd ui && bun --bun run dev
wasm:
    wasm-pack build --target web --no-default-features --features wasm && rm -rf webapp/lib/pkg && mv pkg webapp/lib/pkg
install-py:
    wine64 pip install -r requirements.txt
py:
    #wine64 pip install -r requirements.txt
    wine64 fastapi dev app.py
download-zorro:
    curl https://opserver.de/down/Zorro_setup.exe -o zorrosetup.exe
install-zorro:
    wine64 zorrosetup.exe
download:
    curl https://opserver.de/down/Zorro_setup.exe -o zorro.exe
    curl https://www.python.org/ftp/python/3.12.4/python-3.12.4-amd64.exe -o pythonsetup.exe
    curl https://download.mql5.com/cdn/web/ftmo.s.r/mt5/ftmo5setup.exe -o ftmosetup.exe
install-mt5:
    curl https://www.python.org/ftp/python/3.12.4/python-3.12.4-amd64.exe -o pythonsetup.exe
    curl https://download.mql5.com/cdn/web/ftmo.s.r/mt5/ftmo5setup.exe -o ftmosetup.exe
    wine64 pythonsetup.exe
    wine64 ftmosetup.exe
solana-catchup:
    solana catchup --our-localhost --follow