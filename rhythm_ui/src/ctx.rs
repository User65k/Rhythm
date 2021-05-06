use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::{log, WebRes, show_error_w_val};
use web_sys::{Event, Element, MouseEvent, window, HtmlElement, AddEventListenerOptions, Document};

/// event handler that opens the context menu for a row in the request list
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

            let ctx_fn = Closure::once_into_js(move |_e: Event| {
                menu.style().set_property("display", "none");
            });
            
            window.add_event_listener_with_callback_and_add_event_listener_options(
                "click",
                ctx_fn.unchecked_ref(),
                AddEventListenerOptions::new().once(true)
            )?;
        }
        Ok(())
    }() {
        show_error_w_val("error showing ctx men: ", e);
    }
}
pub fn setup_ctx_men(document: &Document) -> WebRes
{
    //setup callback
    let ctx_fn = Closure::wrap(Box::new(context_menu) as Box<dyn Fn(Event)>);
    let ucref = unsafe {
        CTX_MEN_OPEN = Some(ctx_fn.into_js_value());
        CTX_MEN_OPEN.as_ref().unwrap().unchecked_ref()
    };

    //add callback to all rows in the request list
    let nodes = document.query_selector_all(".ilist tbody tr")?;
    let mut x = 0;
    while x < nodes.length() {
        let n = nodes.get(x).unwrap();
        n.add_event_listener_with_callback(
            "contextmenu",
            ucref
        )?;
        x+= 1;
    }
    Ok(())
}

static mut CTX_MEN_OPEN: Option<JsValue> = None;

pub fn add_ctx_men(ele: &Element) -> WebRes {
    let ucref = unsafe {
        CTX_MEN_OPEN.as_ref().expect("add ctx called before init").unchecked_ref()
    };
    ele.add_event_listener_with_callback(
        "contextmenu",
        ucref
    )?;
    Ok(())
}