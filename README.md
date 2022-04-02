[![Project Status: WIP – Initial development is in progress, but there has not yet been a stable, usable release suitable for the public.](https://www.repostatus.org/badges/latest/wip.svg)](https://www.repostatus.org/#wip)

# A intercepting Proxy in Rust

rust in the middle -> ritm -> "Rhythm"

## Why?
Java Proxys eat your RAM and look ugly while doing it.

Also, ZAP messes up base64 decoding and is not able to alter all the filds in a HTTP request (like the Host).
Furthermore, Burp and esspecially ZAP could use some proxychains style proxy support.

## build

1. https://docs.rs/openssl/latest/openssl/#building
2. https://rustwasm.github.io/wasm-pack/installer/
3. `cargo build -p rhythm_prx`
4. `cd rhythm_ui`
5. `wasm-pack build --target web`
6. `cd ..`
7. `cp rhythm_ui/pkg/rhythm_ui.js main.js`
8. `cp rhythm_ui/pkg/rhythm_ui_bg.wasm main.wasm`
9. `cat rhythm_ui/assets/{code,ctx_men,ilist,misc,switch,tabs,tree}.css > style.css`
10. `cp rhythm_ui/assets/index.html index.html`

## run

`cargo run -p rhythm_prx`

## Features

- [x] TLS Intercept
    - [x] Generate Certs with Common+Alt Name
    - [ ] Individual Root CA
- [x] TLS Passthrought
- [x] Upstream Proxys
    - [x] HTTP Connect
    - [x] Socks
    - [x] Chainable (Proxychains Style)
    - [ ] with match list (Foxyproxy style)
    - [ ] HTTP without Connect in case of HTTP with a single Proxy
- [x] Hosts entries in the Proxy (change name resolution)
- [x] Transparent Mode (Listen with fake cert @ port + forward it)
- [ ] Breakpoints (ZAP or HTTPToolkit Style)
- [ ] Client Certificates
- [ ] Non HTTP

- [ ] Store Stuff in a Database
- [ ] strip encodings and gzip
- [ ] Resend
- [ ] Alter Requests
  - [ ] Add Cookies from Jar
  - [ ] Update Content-Length
- [x] Gateway Timeout
- [ ] HTTP/2
- [ ] Websockets
- [ ] Session completition (auto add Cookies, Auth-Header and CSRF-Tokens if missing)

- [x] [WASM](https://webassembly.org/) Gui
  - [x] History
  - [ ] Options
  - [ ] Edit Requests / Breakpoints
  - [ ] Requests / Responses
  - [ ] Tool Tips for URL / Base64 / XMLEntities
  - [ ] Page Map

- [ ] Start proxied App (HTTPToolkit Style)
- [ ] Start external Application with Parameters from a Request
- [ ] Exclude from History

- [ ] Plugins via [WASI](https://wasi.dev/)
  - [ ] Passive Scanners
  - [ ] Active Scanners
  - [ ] Burp Plugin bridge
  - [ ] scan - [feroxbuster](https://github.com/epi052/feroxbuster)?
    - [ ] vHosts
    - [ ] dirs
    - [ ] HTTP Methods

