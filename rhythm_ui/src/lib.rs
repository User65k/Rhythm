use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: JsValue);
}

fn inv_scroll(e: web_sys::Element, obs: &web_sys::IntersectionObserver)
{
    log(e.into());
}

fn load_scroll(e: web_sys::NodeList, obs: web_sys::IntersectionObserver) {
    log(JsValue::from_str("jo"));
    let mut x = 0;
    while x < e.length() {
        let n = e.get(x).unwrap().dyn_into::<web_sys::IntersectionObserverEntry>().unwrap();
        if n.is_intersecting(){
            obs.unobserve(&n.target());
            inv_scroll(n.target(), &obs);
        }
        x+= 1;
    }
}

fn context_menu(e: web_sys::EventTarget) {
    if let Ok(ele) = e.dyn_into::<web_sys::Element>() {
        log(ele.into());
    }
}

//#[wasm_bindgen(start)]
#[wasm_bindgen]
pub fn init_ui() {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    setup_inf_scroll(&document);
    setup_ctx_men_scroll(&document);
    let loc = document.location().unwrap();
    if let Err(e) = start_websocket(loc) {
        log(e);
        return;
    }
}

fn setup_inf_scroll(document: &web_sys::Document)
{
    let obs_fn = Closure::wrap(Box::new(load_scroll) as Box<dyn Fn(web_sys::NodeList, web_sys::IntersectionObserver)>);
            
    if let Ok(obs) = web_sys::IntersectionObserver::new_with_options(
        obs_fn.as_ref().unchecked_ref(),
        &web_sys::IntersectionObserverInit::new()
        .root(document.query_selector(".ilist tbody").unwrap().as_ref())
        .root_margin("0px")) {
        obs.observe(&document.query_selector(".ilist tbody tr:first-child").unwrap().unwrap());
        obs.observe(&document.query_selector(".ilist tbody tr:last-child").unwrap().unwrap());
    }
    obs_fn.forget();
}
fn setup_ctx_men_scroll(document: &web_sys::Document)
{
    let ctx_fn = Closure::wrap(Box::new(move |e: web_sys::Event| {
        context_menu(e.target().unwrap());
        e.prevent_default();
    }) as Box<dyn Fn(web_sys::Event)>);
    if let Ok(nodes) = document.query_selector_all(".ilist tbody tr") {
        let mut x = 0;
        while x < nodes.length() {
            let n = nodes.get(x).unwrap();
            n.add_event_listener_with_callback(
                "contextmenu",
                ctx_fn.as_ref().unchecked_ref()
            );
            x+= 1;
        }
    }
    ctx_fn.forget();
}

fn start_websocket(loc: web_sys::Location) -> Result<(), JsValue> {

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
        log(array.into());
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
