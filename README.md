[![Project Status: Concept – Minimal or no implementation has been done yet, or the repository is only intended to be a limited example, demo, or proof-of-concept.](https://www.repostatus.org/badges/latest/concept.svg)](https://www.repostatus.org/#concept)

# A intercepting Proxy in Rust

rust in the middle -> ritm -> "Rhythm"

## Why?
Java Proxys eat your RAM and look ugly while doing it.

Also, ZAP messes up base64 decoding and is not able to alter all the filds in a HTTP request (like the Host).
Furthermore, Burp and esspecially ZAP could use some proxychains style proxy support.

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
- [ ] Transparent Mode (Listen with fake cert @ port + forward it)
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

- [ ] HTML Gui
    - [ ] Tool Tips for URL / Base64 / XMLEntities
    - [ ] Page Map
    - [ ] History
    - [ ] Requests / Responses

- [ ] Start proxied App (HTTPToolkit Style)
- [ ] Start external Application with Parameters from a Request
- [ ] Exclude from History

- [ ] Plugins via [WASI](https://wasi.dev/)
  - [ ] Passive Scanners
  - [ ] Active Scanners
  - [ ] scan - [rustbuster](https://github.com/phra/rustbuster)?
    - [ ] vHosts
    - [ ] dirs
    - [ ] HTTP Methods

## build


```
sudo apt install pkg-config libssl-dev
cargo build -p rhythm_prx

wasm-pack build --target web
```