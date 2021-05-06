use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::{log, WebRes, show_error_w_val, list::{get_list_ref, get_scroll_ref, get_list_top_spacer, get_list_bottom_spacer, adjust_spacer_h, clear_row}};
use web_sys::{Element, IntersectionObserver, HtmlElement, NodeList, IntersectionObserverInit, AddEventListenerOptions, Event, IntersectionObserverEntry};

/*
new + follow:

1. add more height to dummy
2. move first to bottom
3. scroll to it

scroll down / bottom reached:

1. add more height to dummy
1. remove height on bottom dummy
2. move first to bottom

scroll up / top reached

1. remove height on top dummy
1. add height on bottom dummy
2. move bottom to top
*/

fn inv_scroll(e: Element, obs: &IntersectionObserver) -> WebRes
{
    let e = e.dyn_into::<HtmlElement>()?;
    let top = get_list_top_spacer();
    let bottom = get_list_bottom_spacer();

    let last = bottom.parent_element()
                    .and_then(|oe|oe.previous_element_sibling())
                    .and_then(|oe|{
                        oe.dyn_into::<HtmlElement>().ok()
                    }).unwrap();
    let first = top.parent_element()
                    .and_then(|oe|oe.next_element_sibling())
                    .and_then(|oe|{
                        oe.dyn_into::<HtmlElement>().ok()
                    }).unwrap();
    if *top == e {
        //first is now in view -> scrolling up
        let off = top.offset_height();
        if off > 0 {//first is not at the top -> we can load more
            //move last item to the top
            clear_row(&last)?;
            first.before_with_node_1(&last)?;
            adjust_spacer_h(top, -1)?;
            adjust_spacer_h(bottom, 1)?;
        }
    }else if *bottom == e{
        //last is now in view -> scrolling down
        clear_row(&first)?;
        last.after_with_node_1(&first)?;//first is now last
        adjust_spacer_h(bottom, -1)?;
        adjust_spacer_h(top, 1)?;
    }
    obs.observe(&e);
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
            //log(n.into());
            x+= 1;
        }
        Ok(())
    }() {
        show_error_w_val("intersection event err: ", e);
    }
}
pub fn setup_inf_scroll(list_root: &Element) -> WebRes
{
    let scroll_ele = get_scroll_ref();

    let obs_fn = Closure::wrap(Box::new(load_scroll) as Box<dyn Fn(NodeList, IntersectionObserver)>);
    
    let obs = IntersectionObserver::new_with_options(
        obs_fn.as_ref().unchecked_ref(),
        &IntersectionObserverInit::new()
        .root(Some(scroll_ele))
        .root_margin("0px"))?;
    
    obs.observe(get_list_top_spacer());
    obs.observe(get_list_bottom_spacer());

    obs_fn.forget();
/*
    let ctx_fn = Closure::wrap(Box::new(fix_scroll) as Box<dyn Fn(Event)>);
    scroll_ele.add_event_listener_with_callback_and_add_event_listener_options(
        "scroll",
        ctx_fn.as_ref().unchecked_ref(),
        AddEventListenerOptions::new().passive(true)
    )?;
    ctx_fn.forget();*/
    Ok(())
}
/*
fn fix_scroll(e: Event) {
    if let Err(e) = move || -> WebRes {
        if let Some(ele) = e.target() {
            let e = ele.dyn_into::<HtmlElement>()?;
            let pos = e.scroll_top();  // 0 - (scrollHeight-clientHeight)
            let first = get_list_top_spacer().offset_height();
            let last = get_list_bottom_spacer().offset_height();

            let height = e.scroll_height() - e.client_height();

            if pos < first {
                //scrolled past top element
                log(JsValue::from_str("prev all"));
            }else if pos > height-last {
                //scrolled past last element
                log(JsValue::from_str("past all"));
            }
        }
        Ok(())
    }() {
        show_error_w_val("scroll event err: ", e);
    }
}*/