use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::{show_error_w_val, show_error, list::new_item};
use rhythm_proto::WSNotify;
use web_sys::{Location, WebSocket, BinaryType, MessageEvent, ErrorEvent};
use js_sys::{ArrayBuffer, Uint8Array, JsString};

pub fn start_websocket(loc: Location) -> Result<(), JsValue> {

    let mut url = String::from("ws");
    {
        if "https" == loc.protocol()? {
            url.push('s');
        }
        url.push_str("://");
        url.push_str(&loc.host()?);
        url.push_str("/events");
    }

    // Connect to an echo server
    let ws = WebSocket::new(&url)?;
    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    ws.set_binary_type(BinaryType::Arraybuffer);
    // create callback
    let onmessage_callback = Closure::wrap(Box::new(on_msg) as Box<dyn FnMut(MessageEvent)>);
    // set message event handler on WebSocket
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    // forget the callback to keep it alive
    onmessage_callback.forget();

    let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
        show_error_w_val("ws error: ", e.into());
    }) as Box<dyn FnMut(ErrorEvent)>);
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    Ok(())
}

fn on_msg(e: MessageEvent) {
    // Handle difference Text/Binary,...
    if let Ok(abuf) = e.data().dyn_into::<ArrayBuffer>() {
        let array = Uint8Array::new(&abuf);
        //let len = array.byte_length() as usize;
        match WSNotify::parse(&array.to_vec()) {
            Ok(todo) => {
                if let Err(e) = match todo {
                    WSNotify::NewReq{id, method, uri} => new_item(id, method, uri),
                    WSNotify::NewResp{id, status} => {Ok(())},
                    _ => Err(format!("server says: {:?}",&todo).into())
                } {
                    show_error_w_val("error handling ws: ", e);
                }
            },
            Err(e) => show_error(&format!("could not parse ws data: {}",e))
        }
    } else if let Ok(txt) = e.data().dyn_into::<JsString>() {
        let s: String = txt.into();
        show_error(&s);
    } else {
        show_error_w_val("message event, received Unknown:", e.data().into());
    }
}
