use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::log;
use rhythm_proto::WSNotify;

pub fn start_websocket(loc: web_sys::Location) -> Result<(), JsValue> {

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
    let ws = web_sys::WebSocket::new(&url)?;
    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    // create callback
    let onmessage_callback = Closure::wrap(Box::new(on_msg) as Box<dyn FnMut(web_sys::MessageEvent)>);
    // set message event handler on WebSocket
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    // forget the callback to keep it alive
    onmessage_callback.forget();

    let onerror_callback = Closure::wrap(Box::new(move |e: web_sys::ErrorEvent| {
        log(e.into());
    }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    Ok(())
}

fn on_msg(e: web_sys::MessageEvent) {
    // Handle difference Text/Binary,...
    if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
        log(JsValue::from_str("message event, received arraybuffer"));
        let array = js_sys::Uint8Array::new(&abuf);
        //let len = array.byte_length() as usize;
        let todo = WSNotify::parse(&array.to_vec());
        log(format!("server says: {:?}",todo).into());
        /*
    } else if let Ok(blob) = e.data().dyn_into::<js_sys::Blob>() {
        console_log!("message event, received blob: {:?}", blob);
        // better alternative to juggling with FileReader is to use https://crates.io/crates/gloo-file
        let fr = web_sys::FileReader::new().unwrap();
        let fr_c = fr.clone();
        // create onLoadEnd callback
        let onloadend_cb = Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
            let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
            let len = array.byte_length() as usize;
            console_log!("Blob received {}bytes: {:?}", len, array.to_vec());
            // here you can for example use the received image/png data
        })
            as Box<dyn FnMut(web_sys::ProgressEvent)>);
        fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
        fr.read_as_array_buffer(&blob).expect("blob not readable");
        onloadend_cb.forget();*/
    } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
        log(JsValue::from_str("message event, received Text: "));
        log(txt.into());
    } else {
        log(JsValue::from_str("message event, received Unknown:"));
        log(e.data().into());
    }
}
