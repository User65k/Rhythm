use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::log;

fn context_menu(e: web_sys::EventTarget) {
    if let Ok(ele) = e.dyn_into::<web_sys::Element>() {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let menu = document.query_selector("#menu")
            .unwrap().unwrap()
            .dyn_into::<web_sys::HtmlElement>().unwrap();
        
        menu.style().set_property("left", &format!("{}px",ele.client_left()));
        menu.style().set_property("top", &format!("{}px",ele.client_top()));
        menu.style().set_property("display", "");
        log(ele.into());

        let ctx_fn = Closure::wrap(Box::new(move |e: web_sys::Event| {
            menu.style().set_property("display", "none");
        }) as Box<dyn Fn(web_sys::Event)>);
        window.add_event_listener_with_callback_and_add_event_listener_options(
            "click",
            ctx_fn.as_ref().unchecked_ref(),
            web_sys::AddEventListenerOptions::new().once(true)
        );
        ctx_fn.forget();
    }
}
pub fn setup_ctx_men(document: &web_sys::Document)
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