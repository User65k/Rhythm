use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::{log, WebRes};

mod scroll;

pub fn setup(document: &web_sys::Document) -> WebRes
{
    let window = web_sys::window().expect("no global `window` exists");

    let cbJ = Closure::wrap(Box::new(move |a: JsValue| {
        let a = a.dyn_into::<js_sys::Array>().unwrap();
        log(a.get(2));
    }) as Box<dyn FnMut(JsValue)>);
    let cb = Closure::wrap(Box::new(move |a: JsValue| {
        let a = a.dyn_into::<web_sys::Response>().unwrap();
        if !a.ok() {
            log("http err".into());
            return;
        }
        match a.json() {
            Ok(a) => {a.then(&cbJ);},
            Err(a) => log(a),
        };        
    }) as Box<dyn FnMut(JsValue)>);
    window.fetch_with_str("/api?op=Brief&id=1").then(&cb);
    cb.forget();

    //if len > 20
    //element.child_element_count(&self) -> u32
    scroll::setup_inf_scroll(document)
}

fn create_row(document: &web_sys::Document) -> Result<web_sys::Element, JsValue> {
    let row: web_sys::Element = document.create_element("tr")?;
    let id   = document.create_element("td")?;
    let time = document.create_element("td")?;
    let meth = document.create_element("td")?;
    let host = document.create_element("td")?;
    let path = document.create_element("td")?;
    let code = document.create_element("td")?;
    let rtt =  document.create_element("td")?;
    let size = document.create_element("td")?;
    let tags = document.create_element("td")?;
    
    row.append_with_node_5(&id, &time, &meth, &host, &path)?;
    row.append_with_node_4(&code, &rtt, &size, &tags)?;
    //row.set_class_name("");
    Ok(row)
}
fn clear_row(row: &web_sys::Element) -> WebRes {
    let c: web_sys::HtmlCollection = row.children();
    let id   = c.item(0).unwrap().dyn_into::<web_sys::HtmlElement>()?;
    let time = c.item(1).unwrap().dyn_into::<web_sys::HtmlElement>()?;
    let meth = c.item(2).unwrap().dyn_into::<web_sys::HtmlElement>()?;
    let host = c.item(3).unwrap().dyn_into::<web_sys::HtmlElement>()?;
    let path = c.item(4).unwrap().dyn_into::<web_sys::HtmlElement>()?;
    let code = c.item(5).unwrap().dyn_into::<web_sys::HtmlElement>()?;
    let rtt  = c.item(6).unwrap().dyn_into::<web_sys::HtmlElement>()?;
    let size = c.item(7).unwrap().dyn_into::<web_sys::HtmlElement>()?;
    let tags = c.item(8).unwrap().dyn_into::<web_sys::HtmlElement>()?;
    id.set_inner_text("███");
    time.set_inner_text("███");
    meth.set_inner_text("███");
    host.set_inner_text("█████");
    path.set_inner_text("█▆▆▆▆▆▃▆▆▆");
    code.set_inner_text("████");
    rtt.set_inner_text("█▃██");
    size.set_inner_text("██▙");
    tags.set_inner_text("");
    meth.set_class_name("c");
    code.set_class_name("c");
    Ok(())
}
