use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::log;

fn inv_scroll(e: web_sys::Element, obs: &web_sys::IntersectionObserver)
{
    log(e.into());
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
pub fn setup_inf_scroll(document: &web_sys::Document)
{
    let obs_fn = Closure::wrap(Box::new(load_scroll) as Box<dyn Fn(web_sys::NodeList, web_sys::IntersectionObserver)>);
            
    if let Ok(obs) = web_sys::IntersectionObserver::new_with_options(
        obs_fn.as_ref().unchecked_ref(),
        &web_sys::IntersectionObserverInit::new()
        .root(document.query_selector(".ilist tbody").unwrap().as_ref())
        .root_margin("0px")) {
        obs.observe(&document.query_selector(".ilist tbody tr:first-child").unwrap().unwrap());
        obs.observe(&document.query_selector(".ilist tbody tr:last-child").unwrap().unwrap());
    }
    obs_fn.forget();
}