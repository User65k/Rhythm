use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::{log, WebRes};

fn inv_scroll(e: web_sys::Element, obs: &web_sys::IntersectionObserver) -> WebRes
{
    //get first and last element of the list
    let p = e.parent_node().unwrap().dyn_into::<web_sys::Element>()?;
    let (first, last) = (p.first_element_child().unwrap(), p.last_element_child().unwrap());
    
    let e = e.dyn_into::<web_sys::HtmlElement>()?;
    let first = first.dyn_into::<web_sys::HtmlElement>()?;
    let last = last.dyn_into::<web_sys::HtmlElement>()?;

    if first == e {
        //first is now in view -> scrolling up
        let off = first.offset_top()/first.offset_height();
        if (off > 0) {//first is not at the top -> we can load more
            //move last item to the top
            obs.observe(&last.previous_element_sibling().unwrap());
            first.before_with_node_1(&last)?;
            last.style().set_property("top",&format!("{}em", off-1))?;
        }
    }else if last == e{
        //last is now in view -> scrolling down
        obs.observe(&first.next_element_sibling().unwrap()); //2nd will be first
        last.after_with_node_1(&first)?;//first is now last
        let off = last.offset_top()/last.offset_height();
        if first.offset_top() == 0 {
            //first could not have a handler
            obs.observe(&first);
        }
        first.style().set_property("top",&format!("{}em", 1+off))?;
    }
    Ok(())
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
pub fn setup_inf_scroll(document: &web_sys::Document) -> WebRes
{
    let obs_fn = Closure::wrap(Box::new(load_scroll) as Box<dyn Fn(web_sys::NodeList, web_sys::IntersectionObserver)>);
    
    let list_root = document.query_selector(".ilist tbody")?;

    if let Ok(obs) = web_sys::IntersectionObserver::new_with_options(
        obs_fn.as_ref().unchecked_ref(),
        &web_sys::IntersectionObserverInit::new()
        .root(list_root.as_ref())
        .root_margin("0px")) {
        obs.observe(&document.query_selector(".ilist tbody tr:first-child")?.unwrap());
        obs.observe(&document.query_selector(".ilist tbody tr:last-child")?.unwrap());
    }
    obs_fn.forget();


    let ctx_fn = Closure::wrap(Box::new(move |e: web_sys::Event| {
        //TODO
    }) as Box<dyn Fn(web_sys::Event)>);
    list_root.unwrap().add_event_listener_with_callback_and_add_event_listener_options(
        "scroll",
        ctx_fn.as_ref().unchecked_ref(),
        web_sys::AddEventListenerOptions::new().passive(true)
    )?;
    ctx_fn.forget();
    Ok(())
}