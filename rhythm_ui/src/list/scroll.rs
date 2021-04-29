use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::{log, WebRes, show_error_w_val};
use web_sys::{Element, IntersectionObserver, HtmlElement, NodeList, IntersectionObserverInit, AddEventListenerOptions, Event, IntersectionObserverEntry};

fn inv_scroll(e: Element, obs: &IntersectionObserver) -> WebRes
{
    //get first and last element of the list
    let p = e.parent_node().unwrap().dyn_into::<Element>()?;
    let (first, last) = (p.first_element_child().unwrap(), p.last_element_child().unwrap());
    
    let e = e.dyn_into::<HtmlElement>()?;
    let first = first.dyn_into::<HtmlElement>()?;
    let last = last.dyn_into::<HtmlElement>()?;

    if first == e {
        //first is now in view -> scrolling up
        let off = first.offset_top()/first.offset_height();
        if off > 0 {//first is not at the top -> we can load more
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
    log(JsValue::from_str("jo"));
    Ok(())
}

fn load_scroll(e: NodeList, obs: IntersectionObserver) {
    if let Err(e) = move || -> WebRes {
        let mut x = 0;
        while x < e.length() {
            let n = e.get(x).unwrap().dyn_into::<IntersectionObserverEntry>()?;
            if n.is_intersecting(){
                obs.unobserve(&n.target());
                inv_scroll(n.target(), &obs)?;
            }
            x+= 1;
        }
        Ok(())
    }() {
        show_error_w_val("intersection event err: ", e);
    }
}
pub fn setup_inf_scroll(list_root: &Element) -> WebRes
{
    let obs_fn = Closure::wrap(Box::new(load_scroll) as Box<dyn Fn(NodeList, IntersectionObserver)>);
    
    let obs = IntersectionObserver::new_with_options(
        obs_fn.as_ref().unchecked_ref(),
        &IntersectionObserverInit::new()
        .root(Some(list_root))
        .root_margin("0px"))?;
    
    obs.observe(&list_root.first_element_child().unwrap());
    obs.observe(&list_root.last_element_child().unwrap());

    obs_fn.forget();


    let ctx_fn = Closure::wrap(Box::new(fix_scroll) as Box<dyn Fn(Event)>);
    list_root.add_event_listener_with_callback_and_add_event_listener_options(
        "scroll",
        ctx_fn.as_ref().unchecked_ref(),
        AddEventListenerOptions::new().passive(true)
    )?;
    ctx_fn.forget();
    Ok(())
}
fn fix_scroll(e: Event) {
    if let Err(e) = move || -> WebRes {
        if let Some(ele) = e.target() {
            let e = ele.dyn_into::<HtmlElement>()?;
            let pos = e.scroll_top();
            let (first, last) = (e.first_element_child().unwrap(), e.last_element_child().unwrap());
            let first = first.dyn_into::<HtmlElement>()?;
            let last = last.dyn_into::<HtmlElement>()?;

            log(JsValue::from_str(&format!("scroll {} < {} < {}",
                first.offset_top(),
                pos,
                last.offset_top()+last.offset_height()-e.offset_height()
            )));

            if pos < first.offset_top()-last.offset_height() {
                //scrolled past top element
                log(JsValue::from_str("prev all"));
            }else if pos + e.offset_height() > last.offset_top()+2*last.offset_height() {
                //scrolled past last element
                log(JsValue::from_str("past all"));
            }
        }
        Ok(())
    }() {
        show_error_w_val("scroll event err: ", e);
    }
}