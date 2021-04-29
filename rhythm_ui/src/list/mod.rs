use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::{log, WebRes, get_document_ref};
use web_sys::{Url, Element, Document, window, Response, HtmlElement};

mod scroll;

static mut LIST_ELE: Option<Element> = None;

fn get_list_ref() -> &'static Element {
    unsafe {
        LIST_ELE.as_ref().expect("No list root")
    }
}

pub fn setup(document: &Document) -> WebRes
{
    let window = window().expect("no global `window` exists");

    let cb_j = Closure::wrap(Box::new(move |a: JsValue| {
        let a = a.dyn_into::<js_sys::Array>().unwrap();
        log(a.get(3));
    }) as Box<dyn FnMut(JsValue)>);
    let cb = Closure::wrap(Box::new(move |a: JsValue| {
        let a = a.dyn_into::<Response>().unwrap();
        if !a.ok() {
            log("http err".into());
            return;
        }
        match a.json() {
            Ok(a) => {a.then(&cb_j);},
            Err(a) => log(a),
        };        
    }) as Box<dyn FnMut(JsValue)>);
    window.fetch_with_str("/api?op=Brief&id=1").then(&cb);
    cb.forget();

    unsafe {
        LIST_ELE = document.query_selector(".ilist tbody")?;
    }
    Ok(())
}
pub fn new_item(id: u64, method: String, uri: String) -> WebRes {
    log(format!("new req {} {} {}", &id, &method, &uri).into());
    let list = get_list_ref();
    let list_elements = list.child_element_count();
    if list_elements > 19 {
        //recycle old rows
        //TODO check if auto scroll is on
        Ok(())
    }else{
        //add new rows
        let doc = get_document_ref();
        let row = create_row(doc)?;
        clear_row(&row)?;
        let c = row.children();
        let cid   = c.item(0).unwrap().dyn_into::<HtmlElement>()?;
        let meth = c.item(2).unwrap().dyn_into::<HtmlElement>()?;
        let host = c.item(3).unwrap().dyn_into::<HtmlElement>()?;
        let path = c.item(4).unwrap().dyn_into::<HtmlElement>()?;
        cid.set_inner_text(&format!("{}",id));
        meth.set_inner_text(&method);

        let uri = Url::new(&uri)?;

        host.set_inner_text(&uri.origin());
        path.set_inner_text(&uri.pathname());
        
        let row = row.dyn_into::<HtmlElement>()?;
        if list_elements == 0 {
            row.style().set_property("top","0px")?;
        }else{
            row.style().set_property("top",&format!("{}em", list_elements))?;
        }        

        list.append_with_node_1(&row)?;
        //time to setup infinit scrolling?
        if list_elements > 19 {
            scroll::setup_inf_scroll(list)
        }else{
            Ok(())
        }
    }
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
