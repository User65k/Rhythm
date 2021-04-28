use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::{log, WebRes};

fn context_menu(evt: web_sys::Event) -> WebRes {
    if let Some(ele) = evt.target() {
        let ele = ele.dyn_into::<web_sys::Element>()?;
        let evnt = evt.dyn_into::<web_sys::MouseEvent>()?;

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let menu = document.query_selector("#menu")?.unwrap()
            .dyn_into::<web_sys::HtmlElement>()?;
        
        menu.style().set_property("left", &format!("{}px", evnt.client_x()))?;
        menu.style().set_property("top", &format!("{}px", evnt.client_y()))?;
        menu.style().set_property("display", "")?;
        log(ele.into());

        let ctx_fn = Closure::wrap(Box::new(move |e: web_sys::Event| {
            menu.style().set_property("display", "none");
        }) as Box<dyn Fn(web_sys::Event)>);
        window.add_event_listener_with_callback_and_add_event_listener_options(
            "click",
            ctx_fn.as_ref().unchecked_ref(),
            web_sys::AddEventListenerOptions::new().once(true)
        )?;
        ctx_fn.forget();
    }
    Ok(())
}
pub fn setup_ctx_men(document: &web_sys::Document) -> WebRes
{
    let ctx_fn = Closure::wrap(Box::new(move |e: web_sys::Event| {
        e.prevent_default();
        if let Err(e) = context_menu(e) {
            log(e);
        }
    }) as Box<dyn Fn(web_sys::Event)>);
    let nodes = document.query_selector_all(".ilist tbody tr")?;
    let mut x = 0;
    while x < nodes.length() {
        let n = nodes.get(x).unwrap();
        n.add_event_listener_with_callback(
            "contextmenu",
            ctx_fn.as_ref().unchecked_ref()
        )?;
        x+= 1;
    }
    ctx_fn.forget();
    Ok(())
}