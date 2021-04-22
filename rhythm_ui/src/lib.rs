use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

mod ws;
mod ctx;
mod scroll;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: JsValue);
}

//#[wasm_bindgen(start)]
#[wasm_bindgen]
pub fn init_ui() {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    scroll::setup_inf_scroll(&document);
    ctx::setup_ctx_men(&document);
    let loc = document.location().unwrap();
    if let Err(e) = ws::start_websocket(loc) {
        log(e);
        return;
    }
}

