use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::{log, WebRes, show_error_w_val};
use web_sys::{Event, Element, MouseEvent, window, HtmlElement, AddEventListenerOptions, Document};

fn context_menu(evt: Event) {
    evt.prevent_default();
    if let Err(e) = move || -> WebRes {
        if let Some(ele) = evt.target() {
            let ele = ele.dyn_into::<Element>()?;
            let evnt = evt.dyn_into::<MouseEvent>()?;

            let window = window().unwrap();
            let document = window.document().unwrap();
            let menu = document.query_selector("#menu")?.unwrap()
                .dyn_into::<HtmlElement>()?;
            
            menu.style().set_property("left", &format!("{}px", evnt.client_x()))?;
            menu.style().set_property("top", &format!("{}px", evnt.client_y()))?;
            menu.style().set_property("display", "")?;
            log(ele.into());

            let ctx_fn = Closure::wrap(Box::new(move |_e: Event| {
                menu.style().set_property("display", "none");
            }) as Box<dyn Fn(Event)>);
            window.add_event_listener_with_callback_and_add_event_listener_options(
                "click",
                ctx_fn.as_ref().unchecked_ref(),
                AddEventListenerOptions::new().once(true)
            )?;
            ctx_fn.forget();
        }
        Ok(())
    }() {
        show_error_w_val("error showing ctx men: ", e);
    }
}
pub fn setup_ctx_men(document: &Document) -> WebRes
{
    let ctx_fn = Closure::wrap(Box::new(context_menu) as Box<dyn Fn(Event)>);
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