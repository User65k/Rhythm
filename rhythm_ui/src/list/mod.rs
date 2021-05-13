use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::{log, WebRes, get_document_ref, ctx::add_ctx_men};
use web_sys::{Url, Element, Document, window, Response, HtmlElement};

mod scroll;

static mut LIST_ELE: Option<HtmlElement> = None;
static mut SCROL_ELE: Option<HtmlElement> = None;
static mut ITEM_HIGHT: Option<i32> = None;
static mut TOP_SPACER: Option<HtmlElement> = None;
static mut BOTTOM_SPACER: Option<HtmlElement> = None;

/// return the list / parent of the rows. (Is a tbody)
fn get_list_ref() -> &'static HtmlElement {
    unsafe {
        LIST_ELE.as_ref().expect("No list root")
    }
}
/// return the element that scrolls the list. (Is a div)
fn get_scroll_ref() -> &'static HtmlElement {
    unsafe {
        SCROL_ELE.as_ref().expect("No scroll ele")
    }
}
/// return the hight in px of a single row (tr) in the list
fn get_item_hight() -> i32 {
    match unsafe {ITEM_HIGHT} {
        Some(h) => h,
        None => {
            let list = get_list_ref();
            let first_row = list.first_element_child().unwrap();
            let row = first_row.next_element_sibling().unwrap();
            let td = row.first_element_child().unwrap();
            let td = td.dyn_into::<HtmlElement>().unwrap();
            let h = td.offset_height();
            unsafe {ITEM_HIGHT = Some(h);}
            h
        }
    }
}
fn get_list_top_spacer() -> &'static HtmlElement {
    unsafe {
        TOP_SPACER.as_ref().expect("No list root")
    }
}
fn get_list_bottom_spacer() -> &'static HtmlElement {
    unsafe {
        BOTTOM_SPACER.as_ref().expect("No list root")
    }
}
fn adjust_spacer_h(element: &HtmlElement, h: i32) -> WebRes
{
    element.style().set_property("height",
        &format!("{}px",
            (h*get_item_hight())+element.offset_height()))?;

    if h<1 && element.offset_height() < 1 {       
        element.parent_element().unwrap().dyn_into::<HtmlElement>()?.style().set_property("display","none")?;
    }else{
        //stops the intersection observer
        element.parent_element().unwrap().dyn_into::<HtmlElement>()?.style().set_property("display","")?;
    }
    Ok(())
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
        LIST_ELE =  document.query_selector(".ilist tbody")?.and_then(|oe|{
            oe.dyn_into::<HtmlElement>().ok()
        });
        SCROL_ELE = LIST_ELE.as_ref()
            .and_then(|oe|oe.parent_element())
            .and_then(|oe|oe.parent_element())
            .and_then(|oe|{
                oe.dyn_into::<HtmlElement>().ok()
            });
        TOP_SPACER = LIST_ELE.as_ref()
            .and_then(|oe|oe.first_element_child())
            .and_then(|oe|oe.first_element_child())
            .and_then(|oe|{
                oe.dyn_into::<HtmlElement>().ok()
            });
        BOTTOM_SPACER = LIST_ELE.as_ref()
            .and_then(|oe|oe.last_element_child())
            .and_then(|oe|oe.first_element_child())
            .and_then(|oe|{
                oe.dyn_into::<HtmlElement>().ok()
            });
    }
    Ok(())
}
/// add new data to the list
pub fn new_item(id: u64, method: String, uri: String) -> WebRes {
    let list = get_list_ref();
    let list_elements = list.child_element_count()-2;
    let s = get_scroll_ref();
    let scroll_down = if s.scroll_height() > s.client_height() {
        //log(format!("scrolled at {}-{}-{} <= {}", s.scroll_height(), s.scroll_top(), get_item_hight(), s.client_height()).into());
        s.scroll_height() - s.scroll_top() - get_item_hight()  <= s.client_height()
    }else{
        false
    };
    let row = if list_elements > 19 {
        //check if auto scroll is on
        if scroll_down {
            //recycle old rows
            let first_row = list.first_element_child().unwrap();
            //1. add more height to dummy
            let td = first_row.first_element_child().unwrap();
            let td = td.dyn_into::<HtmlElement>()?;
            adjust_spacer_h(&td, 1)?;
            //2. move first to bottom
            let row = first_row.next_element_sibling().unwrap();
            let last_row = list.last_element_child().unwrap();
            last_row.before_with_node_1(&row)?;
            //3. scroll to it
            row
        }else{
            //only indicate new elements
            let td = get_list_bottom_spacer();
            adjust_spacer_h(td, 1)?;
            return Ok(());
        }        
    }else{
        //add new rows
        let doc = get_document_ref();
        let row = create_row(doc)?;
        add_ctx_men(&row)?;
        let last_row = list.last_element_child().unwrap();
        last_row.before_with_node_1(&row)?;
        
        //time to setup infinit scrolling?
        if list_elements == 19 {
            scroll::setup_inf_scroll(list)?;
        }
        row
    };
    if scroll_down {
        s.scroll_to_with_x_and_y(0.0, s.scroll_height() as f64);
    }
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
    Ok(())
}

/// create a new list row
/// It is not attached to the DOM yet
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
/// replace all text in a row with some placeholder text
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
